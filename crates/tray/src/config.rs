use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::Result;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use notify_debouncer_full::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProcessMode {
    Copy,
    #[default]
    Paste,
    Send,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub current_character: String,
    #[serde(default)]
    pub process_mode: ProcessMode,
    #[serde(default)]
    pub intercept_enter: bool,
    #[serde(default = "default_enable_whitelist")]
    pub enable_whitelist: bool,
    #[serde(default = "default_whitelist")]
    pub whitelist: Vec<String>,
    #[serde(default = "default_max_image_size")]
    pub max_image_size: usize,
    #[serde(default = "default_max_chars")]
    pub max_chars: usize,
    #[serde(default = "default_toggle_hotkey")]
    pub toggle_hotkey: HotKey,
    #[serde(default = "default_generate_hotkey")]
    pub generate_hotkey: HotKey,
}

fn default_enable_whitelist() -> bool {
    true
}

fn default_whitelist() -> Vec<String> {
    vec![
        "WeChat".to_string(),
        "Weixin".to_string(),
        "QQ".to_string(),
        "TIM".to_string(),
    ]
}

fn default_max_image_size() -> usize {
    256
}

fn default_max_chars() -> usize {
    50
}

fn default_toggle_hotkey() -> HotKey {
    HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyT)
}

fn default_generate_hotkey() -> HotKey {
    HotKey::new(Some(Modifiers::CONTROL), Code::KeyE)
}

impl Default for Config {
    fn default() -> Self {
        Config {
            current_character: String::new(),
            process_mode: ProcessMode::default(),
            intercept_enter: false,
            enable_whitelist: true,
            whitelist: default_whitelist(),
            max_image_size: default_max_image_size(),
            max_chars: default_max_chars(),
            toggle_hotkey: default_toggle_hotkey(),
            generate_hotkey: default_generate_hotkey(),
        }
    }
}

impl Config {
    fn load(config_path: &PathBuf) -> Self {
        if config_path.exists()
            && let Ok(content) = fs::read_to_string(config_path)
            && let Ok(config) = serde_yaml_ng::from_str(&content)
        {
            config
        } else {
            Config::default()
        }
    }

    fn save(&self, config_path: &PathBuf) -> Result<()> {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let yaml = serde_yaml_ng::to_string(self)?;
        fs::write(config_path, yaml)?;
        Ok(())
    }
}

pub struct ConfigManager {
    config_path: PathBuf,
    config: Config,
    needs_reload: Arc<AtomicBool>,
    _watcher: Debouncer<RecommendedWatcher, RecommendedCache>,
}

impl ConfigManager {
    pub fn new(config_path: PathBuf) -> Result<Self> {
        let config = Config::load(&config_path);

        let needs_reload = Arc::new(AtomicBool::new(false));
        let needs_reload_clone = needs_reload.clone();

        let current_dir = config_path.parent().unwrap().to_path_buf();

        let watch_path = config_path.clone();
        let mut watcher = new_debouncer(
            Duration::from_millis(500),
            None,
            move |result: DebounceEventResult| {
                if let Ok(events) = result {
                    for event in events {
                        for path in &event.paths {
                            if path == &watch_path {
                                needs_reload_clone.store(true, Ordering::Relaxed);
                                break;
                            }
                        }
                    }
                }
            },
        )?;

        watcher.watch(&current_dir, RecursiveMode::NonRecursive)?;

        let manager = Self {
            config_path,
            config,
            needs_reload,
            _watcher: watcher,
        };
        manager.config.save(&manager.config_path).ok();
        Ok(manager)
    }

    pub fn get_config(&self) -> &Config {
        &self.config
    }

    pub fn try_reload(&mut self) -> bool {
        if self.needs_reload.swap(false, Ordering::Relaxed) {
            self.config = Config::load(&self.config_path);
            self.config.save(&self.config_path).ok();
            true
        } else {
            false
        }
    }

    pub fn set_current_character(&mut self, character: String) -> Result<()> {
        self.config.current_character = character;
        self.config.save(&self.config_path)
    }

    pub fn set_process_mode(&mut self, mode: ProcessMode) -> Result<()> {
        self.config.process_mode = mode;
        self.config.save(&self.config_path)
    }

    pub fn set_intercept_enter(&mut self, enabled: bool) -> Result<()> {
        self.config.intercept_enter = enabled;
        self.config.save(&self.config_path)
    }

    pub fn set_enable_whitelist(&mut self, enabled: bool) -> Result<()> {
        self.config.enable_whitelist = enabled;
        self.config.save(&self.config_path)
    }
}
