use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};

use crate::data::{BLACK, CharacterData, DataConfig, Object};

pub struct DataManager {
    data_dir: PathBuf,
    pub character_configs: HashMap<String, CharacterData>,
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

    pub fn get_backgrounds(&self, character_id: &str) -> Option<Vec<PathBuf>> {
        let character_config = self.character_configs.get(character_id)?;
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

    pub fn get_character_images(
        &self,
        character_id: &str,
    ) -> Option<HashMap<String, Vec<PathBuf>>> {
        let character_config = self.character_configs.get(character_id)?;

        let mut image_patterns = Vec::new();
        for object in &character_config.objects {
            if let Object::Image { path, .. } = object {
                image_patterns.extend(path.clone());
            }
        }

        image_patterns.sort_unstable();
        image_patterns.dedup();

        let images_dir = self.data_dir.join("images");
        let mut character_imgs = HashMap::new();
        for pattern in &image_patterns {
            let resolved_pattern = pattern.replace("%c", character_id);
            let paths = collect_image_paths(&images_dir, &resolved_pattern);
            character_imgs.insert(pattern.clone(), paths);
        }

        Some(character_imgs)
    }

    pub fn get_font_path(&self, character_id: &str) -> Option<PathBuf> {
        let character_config = self.character_configs.get(character_id)?;
        Some(self.data_dir.join("fonts").join(&character_config.font))
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

pub fn load_data(content: &str) -> Result<HashMap<String, CharacterData>> {
    let config = serde_json::from_str::<DataConfig>(content).context("解析资源配置失败")?;

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
            bail!("角色 '{}' 缺少 backgrounds 配置", key);
        }

        backgrounds.sort_unstable();
        backgrounds.dedup();

        let font = raw_character
            .font
            .or_else(|| template.font.clone())
            .ok_or_else(|| anyhow!("角色 '{}' 缺少 font 配置", key))?;

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
            .ok_or_else(|| anyhow!("角色 '{}' 缺少 textarea 配置", key))?;

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

    Ok(result)
}
