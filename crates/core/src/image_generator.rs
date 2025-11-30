use std::io::Cursor;
use std::path::PathBuf;

use ab_glyph::{FontVec, PxScale};
use anyhow::{Result, anyhow};
use image::{ImageFormat, Rgba, RgbaImage, imageops};
use imageproc::drawing::draw_text_mut;

use crate::data::{HorizontalAlign, ObjectConfig, TextAreaConfig, VerticalAlign};
use crate::data_manager::DataManager;
use crate::resource_loader::{load_font, load_random_image};
use crate::textarea::prepare_textarea;

// 压缩保守系数
const CONSERVATIVE_FACTOR: f32 = 0.9;

#[allow(clippy::too_many_arguments)]
fn draw_text_with_shadow(
    image: &mut RgbaImage,
    text: &str,
    x: i32,
    y: i32,
    font: &FontVec,
    scale: PxScale,
    color: Rgba<u8>,
    shadow_offset: (i32, i32),
) {
    let scale = PxScale {
        x: scale.x * 1.3,
        y: scale.y * 1.3,
    };

    // 绘制阴影
    let shadow_color = Rgba([0u8, 0u8, 0u8, 255u8]);
    draw_text_mut(
        image,
        shadow_color,
        x + shadow_offset.0,
        y + shadow_offset.1,
        scale,
        font,
        text,
    );

    // 绘制主文字
    draw_text_mut(image, color, x, y, scale, font, text);
}

fn draw_textarea(
    image: &mut RgbaImage,
    text: &str,
    font: &FontVec,
    config: &TextAreaConfig,
    primary_color: Rgba<u8>,
) {
    let x1 = config.position[0];
    let y1 = config.position[1];
    let x2 = x1 + config.size[0] as i32;
    let y2 = y1 + config.size[1] as i32;
    let normal_color = config.font_color.to_rgba(primary_color);
    let highlight_color = config.highlight.as_ref().map(|c| c.to_rgba(primary_color));

    // 准备文本区域
    let prepared = prepare_textarea(
        text,
        font,
        config.size[0],
        config.size[1],
        config.max_font_size,
        config.line_spacing,
    );

    let scale = PxScale::from(prepared.font_size as f32);

    // 垂直对齐
    let y_start = match &config.valign {
        VerticalAlign::Top => y1,
        VerticalAlign::Middle => y1 + (config.size[1] as i32 - prepared.block_height) / 2,
        VerticalAlign::Bottom => y2 - prepared.block_height,
    };

    // 绘制每一行
    let mut y = y_start;
    for line in &prepared.lines {
        let mut line_width = 0;
        for (_, width) in line {
            line_width += width;
        }

        // 水平对齐
        let mut x = match &config.align {
            HorizontalAlign::Left => x1,
            HorizontalAlign::Center => x1 + (config.size[0] as i32 - line_width) / 2,
            HorizontalAlign::Right => x2 - line_width,
        };

        // 绘制每个文本段
        for (segment, segment_width) in line {
            if !segment.text.is_empty() {
                let color = if segment.is_highlighted
                    && let Some(hl_color) = highlight_color
                {
                    hl_color
                } else {
                    normal_color
                };

                draw_text_with_shadow(
                    image,
                    &segment.text,
                    x,
                    y,
                    font,
                    scale,
                    color,
                    config.shadow_offset,
                );

                x += segment_width;
            }
        }

        y += prepared.spaced_line_height;
        if y >= y2 {
            break;
        }
    }
}

fn compress_image(img: RgbaImage, target_size_bytes: usize) -> RgbaImage {
    let (width, height) = img.dimensions();

    let mut buf = Vec::new();
    if img
        .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
        .is_err()
    {
        return img;
    }

    let original_size = buf.len();

    if original_size <= target_size_bytes {
        return img;
    }

    let size_ratio = (target_size_bytes as f32) / (original_size as f32);
    let scale_factor = size_ratio.sqrt() * CONSERVATIVE_FACTOR;

    let width = ((width as f32 * scale_factor) as u32).max(1);
    let height = ((height as f32 * scale_factor) as u32).max(1);

    imageops::resize(&img, width, height, imageops::FilterType::Lanczos3)
}

pub fn generate_image(
    data_manager: &DataManager,
    character_id: &str,
    text: &str,
    max_size: usize,
) -> Result<RgbaImage> {
    let character_config = data_manager
        .get_character(character_id)
        .ok_or_else(|| anyhow!("角色 '{}' 不存在", character_id))?;

    let mut rng = rand::rng();

    let backgrounds = data_manager
        .get_backgrounds(character_config)
        .ok_or_else(|| anyhow!("角色 '{}' 没有可用的背景图片", character_id))?;
    let backgrounds_vec: Vec<&PathBuf> = backgrounds.iter().collect();
    let mut image = load_random_image(&mut rng, &backgrounds_vec)
        .ok_or_else(|| anyhow!("无法加载角色 '{}' 的背景图片", character_id))?;

    let font_path = data_manager.get_font_path(character_config);
    let font = load_font(&font_path)
        .ok_or_else(|| anyhow!("无法加载角色 '{}' 的字体文件", character_id))?;

    let character_imgs = data_manager.get_character_images(character_config).unwrap();
    for object in &character_config.objects {
        match object {
            ObjectConfig::Image { position, path } => {
                let mut available_imgs = Vec::new();

                for pattern in path {
                    if let Some(imgs) = character_imgs.get(pattern) {
                        available_imgs.extend(imgs);
                    }
                }

                available_imgs.sort_unstable();
                available_imgs.dedup();

                if let Some(img) = load_random_image(&mut rng, &available_imgs) {
                    imageops::overlay(&mut image, &img, position[0] as i64, position[1] as i64);
                }
            }
            ObjectConfig::Text {
                text,
                position,
                font_color,
                font_size,
            } => {
                if !text.is_empty() {
                    let scale = PxScale::from(*font_size as f32);
                    let color = font_color.to_rgba(character_config.primary_color);

                    draw_text_with_shadow(
                        &mut image,
                        text,
                        position[0],
                        position[1],
                        &font,
                        scale,
                        color,
                        (2, 2),
                    );
                }
            }
        }
    }

    draw_textarea(
        &mut image,
        text,
        &font,
        &character_config.textarea,
        character_config.primary_color,
    );

    Ok(if max_size > 0 {
        let max_size = if max_size > usize::MAX / 1024 {
            usize::MAX
        } else {
            max_size * 1024
        };
        compress_image(image, max_size)
    } else {
        image
    })
}
