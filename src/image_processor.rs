use std::io::Cursor;
use std::path::PathBuf;

use ab_glyph::{Font, FontVec, PxScale};
use image::{Rgba, RgbaImage, imageops};
use imageproc::drawing::draw_text_mut;

use crate::data_manager::{DataManager, HorizontalAlign, Object, TextAreaConfig, VerticalAlign};
use crate::loader::{load_font, load_random_image};

// 压缩保守系数
const CONSERVATIVE_FACTOR: f32 = 0.9;

fn measure_text_width(text: &str, font: &FontVec, scale: PxScale) -> i32 {
    let mut width: f32 = 0.0;

    for c in text.chars() {
        let glyph_id = font.glyph_id(c);
        width +=
            font.h_advance_unscaled(glyph_id) * scale.x / font.units_per_em().unwrap_or(1000.0);
    }

    width.ceil() as i32
}

fn get_line_height(font: &FontVec, scale: PxScale) -> i32 {
    let ascent = font.ascent_unscaled();
    let descent = font.descent_unscaled();
    let line_gap = font.line_gap_unscaled();

    ((ascent - descent + line_gap) * scale.y / font.units_per_em().unwrap_or(1000.0)).ceil() as i32
}

fn wrap_text(text: &str, font: &FontVec, scale: PxScale, max_width: i32) -> Vec<String> {
    let mut lines = Vec::new();

    for paragraph in text.lines() {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }

        let chars: Vec<char> = paragraph.chars().collect();
        let mut current_line = String::new();

        for ch in chars {
            let test_line = format!("{}{}", current_line, ch);
            let width = measure_text_width(&test_line, font, scale);

            if width <= max_width {
                current_line.push(ch);
            } else {
                if !current_line.is_empty() {
                    lines.push(current_line.clone());
                }
                current_line = ch.to_string();
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

fn find_best_font_size(
    text: &str,
    font: &FontVec,
    region_width: u32,
    region_height: u32,
    max_font_size: Option<u32>,
    line_spacing: f32,
) -> (u32, Vec<String>, i32, i32) {
    let max_size = if let Some(max_h) = max_font_size {
        max_h.min(region_height)
    } else {
        region_height
    };

    let mut lo = 1u32;
    let mut hi = max_size;
    let mut best_size = 1u32;
    let mut best_lines = vec![text.to_string()];
    let mut best_spaced_line_height = 1i32;
    let mut best_block_height = 1i32;

    while lo <= hi {
        let mid = (lo + hi) / 2;
        let scale = PxScale::from(mid as f32);
        let lines = wrap_text(text, font, scale, region_width as i32);

        // 计算行高和文本块尺寸
        let line_height = get_line_height(font, scale);
        let spaced_line_height = (line_height as f32 * (1.0 + line_spacing)).ceil() as i32;

        let mut max_width = 0;
        for line in &lines {
            let width = measure_text_width(line, font, scale);
            max_width = max_width.max(width);
        }

        let total_height = if lines.is_empty() {
            line_height
        } else {
            spaced_line_height * lines.len() as i32
        };

        if max_width <= region_width as i32 && total_height <= region_height as i32 {
            best_size = mid;
            best_lines = lines;
            best_spaced_line_height = spaced_line_height;
            best_block_height = total_height;
            lo = mid + 1;
        } else {
            hi = mid - 1;
        }
    }

    (
        best_size,
        best_lines,
        best_spaced_line_height,
        best_block_height,
    )
}

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

fn draw_textarea(image: &mut RgbaImage, text: &str, font: &FontVec, config: &TextAreaConfig) {
    let x1 = config.position[0];
    let y1 = config.position[1];
    let x2 = x1 + config.size[0] as i32;
    let y2 = y1 + config.size[1] as i32;
    let color = Rgba([
        config.font_color[0],
        config.font_color[1],
        config.font_color[2],
        255u8,
    ]);

    // 查找最佳字体大小
    let (font_size, lines, spaced_line_height, block_height) = find_best_font_size(
        text,
        font,
        config.size[0],
        config.size[1],
        config.max_font_size,
        config.line_spacing,
    );

    let scale = PxScale::from(font_size as f32);

    // 垂直对齐
    let y_start = match &config.valign {
        VerticalAlign::Top => y1,
        VerticalAlign::Middle => y1 + (config.size[1] as i32 - block_height) / 2,
        VerticalAlign::Bottom => y2 - block_height,
    };

    // 绘制每一行
    let mut y = y_start;
    for line in &lines {
        let line_width = measure_text_width(line, font, scale);

        // 水平对齐
        let x = match &config.align {
            HorizontalAlign::Left => x1,
            HorizontalAlign::Center => x1 + (config.size[0] as i32 - line_width) / 2,
            HorizontalAlign::Right => x2 - line_width,
        };

        draw_text_with_shadow(image, line, x, y, font, scale, color, config.shadow_offset);

        y += spaced_line_height;
        if y >= y2 {
            break;
        }
    }
}

fn compress_image(img: RgbaImage, target_size_bytes: usize) -> RgbaImage {
    let (width, height) = img.dimensions();

    let mut buf = Vec::new();
    if img
        .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
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
    character_name: &str,
    text: &str,
    max_size: usize,
) -> Option<RgbaImage> {
    let character_data = data_manager.character_configs.get(character_name)?;

    let mut rng = rand::rng();

    let backgrounds_vec: Vec<&PathBuf> = data_manager.backgrounds.iter().collect();
    let mut image = load_random_image(&mut rng, &backgrounds_vec)?;

    let font = load_font(&character_data.font)?;

    for object in &character_data.objects {
        match object {
            Object::Image { position, path } => {
                let mut available_imgs = Vec::new();

                for pattern in path {
                    if let Some(imgs) = data_manager.character_imgs.get(pattern) {
                        available_imgs.extend(imgs);
                    }
                }

                available_imgs.sort_unstable();
                available_imgs.dedup();

                if let Some(img) = load_random_image(&mut rng, &available_imgs) {
                    imageops::overlay(&mut image, &img, position[0] as i64, position[1] as i64);
                }
            }
            Object::Text {
                text,
                position,
                font_color,
                font_size,
            } => {
                if !text.is_empty() {
                    let scale = PxScale::from(*font_size as f32);
                    let color = Rgba([font_color[0], font_color[1], font_color[2], 255u8]);

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

    draw_textarea(&mut image, text, &font, &character_data.textarea);

    Some(if max_size > 0 {
        compress_image(image, max_size * 1024)
    } else {
        image
    })
}
