use std::collections::HashMap;

use image::Rgba;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum ColorInput {
    RgbaArr([u8; 4]),
    RgbArr([u8; 3]),
    Literal(String),
}

pub const BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);
pub const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);

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

#[derive(Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum ObjectConfig {
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

#[derive(Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum HorizontalAlign {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum VerticalAlign {
    #[default]
    Top,
    Middle,
    Bottom,
}

#[derive(Deserialize, Serialize, Clone)]
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

#[derive(Clone)]
pub struct CharacterConfig {
    pub id: String,
    pub name: String,
    pub backgrounds: Vec<String>,
    pub font: String,
    pub primary_color: Rgba<u8>,
    pub objects: Vec<ObjectConfig>,
    pub textarea: TextAreaConfig,
}

#[derive(Deserialize, Serialize)]
pub struct Template {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backgrounds: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_color: Option<ColorInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub objects: Option<Vec<ObjectConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub textarea: Option<TextAreaConfig>,
}

#[derive(Deserialize, Serialize)]
pub struct CharacterConfigRaw {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backgrounds: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_color: Option<ColorInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub objects: Option<Vec<ObjectConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub textarea: Option<TextAreaConfig>,
}

#[derive(Deserialize, Serialize)]
pub struct DataConfig {
    pub template: Template,
    pub characters: HashMap<String, CharacterConfigRaw>,
}
