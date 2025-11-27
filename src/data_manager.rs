use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use image::Rgba;
use rfd::MessageDialog;
use rfd::MessageLevel;
use serde::{Deserialize, Serialize};

use crate::config::get_current_dir;

pub fn get_data_dir() -> PathBuf {
    get_current_dir().join("data")
}

#[derive(Clone)]
pub struct DataManager {
    pub backgrounds: Vec<PathBuf>,
    pub character_imgs: HashMap<String, Vec<PathBuf>>,
    pub character_configs: HashMap<String, CharacterData>,
}

impl DataManager {
    pub fn init() -> Self {
        let data_dir = get_data_dir();
        if !data_dir.exists() || !data_dir.is_dir() {
            show_error_and_exit("data 文件夹不存在！");
        }

        let character_configs = load_data_json();
        if character_configs.is_empty() {
            show_error_and_exit("data.json 中没有角色配置！");
        }

        for (name, config) in &character_configs {
            if !get_data_dir().join("fonts").join(&config.font).exists() {
                show_error_and_exit(&format!(
                    "角色 '{}' 的字体文件不存在: {}",
                    name, config.font
                ));
            }
        }

        DataManager {
            backgrounds: Vec::new(),
            character_imgs: HashMap::new(),
            character_configs,
        }
    }

    pub fn switch_to_character(&mut self, character_name: &str) {
        let character_config = if let Some(config) = self.character_configs.get(character_name) {
            config
        } else {
            show_error_and_exit(&format!("角色配置不存在: {}", character_name));
        };

        let background_dir = get_data_dir().join("backgrounds");
        let mut backgrounds = Vec::new();
        for pattern in &character_config.backgrounds {
            let paths = collect_image_paths(background_dir.clone(), pattern);
            backgrounds.extend(paths);
        }

        if backgrounds.is_empty() {
            show_error_and_exit(&format!("角色 '{}' 没有可用的背景图片！", character_name));
        }

        backgrounds.sort_unstable();
        backgrounds.dedup();
        self.backgrounds = backgrounds;

        let mut image_patterns = Vec::new();
        for object in &character_config.objects {
            if let Object::Image { path, .. } = object {
                image_patterns.extend(path.clone());
            }
        }

        image_patterns.sort_unstable();
        image_patterns.dedup();

        let images_dir = get_data_dir().join("images");
        let mut character_imgs = HashMap::new();
        for pattern in &image_patterns {
            let resolved_pattern = pattern.replace("%c", character_name);
            let paths = collect_image_paths(images_dir.clone(), &resolved_pattern);
            character_imgs.insert(pattern.clone(), paths);
        }
        self.character_imgs = character_imgs;
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum ColorInput {
    RgbaArr([u8; 4]),
    RgbArr([u8; 3]),
    Literal(String),
}

const BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);
const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);

impl ColorInput {
    pub fn to_rgba(&self, primary: Rgba<u8>) -> Rgba<u8> {
        match self {
            ColorInput::RgbaArr(c) => Rgba(*c),
            ColorInput::RgbArr([r, g, b]) => Rgba([*r, *g, *b, 255]),
            ColorInput::Literal(s) => match s.as_str() {
                "primary" => primary,
                "white" => WHITE,
                _ => BLACK,
            },
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum Object {
    Text {
        text: String,
        position: [i32; 2],
        font_color: ColorInput,
        font_size: u32,
    },
    Image {
        position: [i32; 2],
        path: Vec<String>,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum HorizontalAlign {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum VerticalAlign {
    #[default]
    Top,
    Middle,
    Bottom,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TextAreaConfig {
    pub position: [i32; 2],
    pub size: [u32; 2],
    pub font_color: ColorInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight: Option<ColorInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_font_size: Option<u32>,
    #[serde(default)]
    pub shadow_offset: (i32, i32),
    #[serde(default)]
    pub line_spacing: f32,
    #[serde(default)]
    pub align: HorizontalAlign,
    #[serde(default)]
    pub valign: VerticalAlign,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CharacterDataRaw {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backgrounds: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_color: Option<ColorInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub objects: Option<Vec<Object>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub textarea: Option<TextAreaConfig>,
}

#[derive(Debug, Clone)]
pub struct CharacterData {
    pub name: String,
    pub backgrounds: Vec<String>,
    pub font: String,
    pub primary_color: Rgba<u8>,
    pub objects: Vec<Object>,
    pub textarea: TextAreaConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TemplateConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backgrounds: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_color: Option<ColorInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub objects: Option<Vec<Object>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub textarea: Option<TextAreaConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DataConfig {
    pub template: TemplateConfig,
    pub characters: HashMap<String, CharacterDataRaw>,
}

fn collect_image_paths(dir: PathBuf, pattern: &str) -> Vec<PathBuf> {
    let mut path_list = Vec::new();

    if !dir.exists() || !dir.is_dir() {
        return path_list;
    }

    let full_pattern = dir.join(pattern).to_string_lossy().to_string();

    if let Ok(paths) = glob::glob(&full_pattern) {
        for entry in paths.flatten() {
            if entry.is_file() {
                path_list.push(entry);
            }
        }
    }

    path_list
}

fn load_data_json() -> HashMap<String, CharacterData> {
    let data_json_path = get_data_dir().join("data.json");

    if !data_json_path.exists() {
        show_error_and_exit("data.json 不存在！");
    }

    match fs::read_to_string(&data_json_path) {
        Ok(content) => match serde_json::from_str::<DataConfig>(&content) {
            Ok(config) => {
                let template = config.template;
                let mut result = HashMap::new();

                for (key, raw_character) in config.characters {
                    let mut backgrounds = Vec::new();
                    if let Some(template_bg) = &template.backgrounds {
                        backgrounds.extend(template_bg.clone());
                    }
                    if let Some(char_bg) = raw_character.backgrounds {
                        backgrounds.extend(char_bg);
                    }

                    if backgrounds.is_empty() {
                        show_error_and_exit(&format!("角色 '{}' 缺少 backgrounds 配置！", key));
                    }

                    backgrounds.sort_unstable();
                    backgrounds.dedup();

                    let font = match raw_character.font.or_else(|| template.font.clone()) {
                        Some(path) => path,
                        None => {
                            show_error_and_exit(&format!("角色 '{}' 缺少 font 配置！", key));
                        }
                    };

                    let primary_color = raw_character
                        .primary_color
                        .or_else(|| template.primary_color.clone())
                        .map(|c| c.to_rgba(BLACK))
                        .unwrap_or(BLACK);

                    let mut objects = template.objects.clone().unwrap_or_else(Vec::new);
                    if let Some(mut char_objects) = raw_character.objects {
                        objects.append(&mut char_objects);
                    }

                    let textarea = match raw_character
                        .textarea
                        .or_else(|| template.textarea.clone())
                    {
                        Some(config) => config,
                        None => {
                            show_error_and_exit(&format!("角色 '{}' 缺少 textarea 配置！", key));
                        }
                    };

                    result.insert(
                        key,
                        CharacterData {
                            name: raw_character.name,
                            backgrounds,
                            font,
                            primary_color,
                            objects,
                            textarea,
                        },
                    );
                }

                result
            }
            Err(e) => {
                show_error_and_exit(&format!("解析 data.json 失败: {:?}", e));
            }
        },
        Err(e) => {
            show_error_and_exit(&format!("读取 data.json 失败: {:?}", e));
        }
    }
}

fn show_error_and_exit(message: &str) -> ! {
    MessageDialog::new()
        .set_level(MessageLevel::Error)
        .set_title("资源加载失败")
        .set_description(message)
        .show();

    std::process::exit(1);
}
