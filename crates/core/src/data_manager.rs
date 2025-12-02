use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};

use crate::data::{BLACK, CharacterConfig, DataConfig};

pub struct DataManager {
    data_dir: PathBuf,
    character_configs: Vec<CharacterConfig>,
}

impl DataManager {
    pub fn new(config_path: &Path) -> Result<Self> {
        if !config_path.exists() || !config_path.is_file() {
            bail!("资源配置文件不存在");
        }

        let content = fs::read_to_string(config_path).context("读取资源配置文件失败")?;
        let character_configs = load_data(&content)?;
        if character_configs.is_empty() {
            bail!("资源配置中没有角色");
        }

        Ok(DataManager {
            data_dir: config_path.parent().unwrap().to_path_buf(),
            character_configs,
        })
    }

    pub fn get_character(&self, character_id: &str) -> Option<&CharacterConfig> {
        self.character_configs.iter().find(|c| c.id == character_id)
    }

    pub fn get_characters(&self) -> &Vec<CharacterConfig> {
        &self.character_configs
    }

    pub(crate) fn get_backgrounds(
        &self,
        character_config: &CharacterConfig,
    ) -> Option<Vec<PathBuf>> {
        let background_dir = self.data_dir.join("backgrounds");
        let mut backgrounds = Vec::new();

        for pattern in &character_config.backgrounds {
            let paths = collect_image_paths(&background_dir, pattern);
            backgrounds.extend(paths);
        }

        if backgrounds.is_empty() {
            return None;
        }

        backgrounds.sort_unstable();
        backgrounds.dedup();
        Some(backgrounds)
    }

    pub(crate) fn get_images(
        &self,
        character_config: &CharacterConfig,
        paths: &[String],
    ) -> Vec<PathBuf> {
        let images_dir = self.data_dir.join("images");
        let mut result = Vec::new();

        for pattern in paths {
            let resolved_pattern = pattern.replace("%c", &character_config.id);
            let image_paths = collect_image_paths(&images_dir, &resolved_pattern);
            result.extend(image_paths);
        }

        result.sort_unstable();
        result.dedup();
        result
    }

    pub(crate) fn get_font_path(&self, character_config: &CharacterConfig) -> PathBuf {
        self.data_dir.join("fonts").join(&character_config.font)
    }
}

fn collect_image_paths(dir: &Path, pattern: &str) -> Vec<PathBuf> {
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

fn load_data(content: &str) -> Result<Vec<CharacterConfig>> {
    let config = serde_json::from_str::<DataConfig>(content).context("解析资源配置失败")?;

    let template = config.template;
    let mut result = Vec::new();

    for (id, raw_character) in config.characters {
        let mut backgrounds = Vec::new();
        if let Some(template_bg) = &template.backgrounds {
            backgrounds.extend(template_bg.clone());
        }
        if let Some(char_bg) = raw_character.backgrounds {
            backgrounds.extend(char_bg);
        }

        if backgrounds.is_empty() {
            bail!("角色 '{}' 缺少 backgrounds 配置", id);
        }

        backgrounds.sort_unstable();
        backgrounds.dedup();

        let font = raw_character
            .font
            .or_else(|| template.font.clone())
            .ok_or_else(|| anyhow!("角色 '{}' 缺少 font 配置", id))?;

        let primary_color = raw_character
            .primary_color
            .or_else(|| template.primary_color.clone())
            .map_or(BLACK, |c| c.to_rgba(BLACK));

        let mut objects = template.objects.clone().unwrap_or_else(Vec::new);
        if let Some(mut char_objects) = raw_character.objects {
            objects.append(&mut char_objects);
        }

        let textarea = raw_character
            .textarea
            .or_else(|| template.textarea.clone())
            .ok_or_else(|| anyhow!("角色 '{}' 缺少 textarea 配置", id))?;

        result.push(CharacterConfig {
            id,
            name: raw_character.name,
            backgrounds,
            font,
            primary_color,
            objects,
            textarea,
        });
    }

    Ok(result)
}
