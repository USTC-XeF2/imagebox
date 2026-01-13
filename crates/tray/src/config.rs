use std::fs;
use std::mem;
use std::path::{Path, PathBuf};
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
    fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config = toml::from_str(&content)?;
        Ok(config)
    }

    fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

pub struct ConfigManager {
    config_path: PathBuf,
    config: Config,
    ignore_next_change: Arc<AtomicBool>,
    _watcher: Debouncer<RecommendedWatcher, RecommendedCache>,
}

impl ConfigManager {
    pub fn new<F>(config_path: PathBuf, on_change: F) -> Result<Self>
    where
        F: Fn() + Send + 'static,
    {
        let config = Config::load(&config_path).unwrap_or_default();

        let ignore_next_change = Arc::new(AtomicBool::new(false));
        let ignore_clone = ignore_next_change.clone();

        let watch_path = config_path.clone();
        let mut watcher = new_debouncer(
            Duration::from_millis(500),
            None,
            move |result: DebounceEventResult| {
                if let Ok(events) = result
                    && events.iter().any(|e| e.paths.contains(&watch_path))
                    && !ignore_clone.swap(false, Ordering::SeqCst)
                {
                    on_change();
                }
            },
        )?;

        watcher.watch(config_path.parent().unwrap(), RecursiveMode::NonRecursive)?;

        let manager = Self {
            config_path,
            config,
            ignore_next_change,
            _watcher: watcher,
        };
        manager.save_config().ok();
        Ok(manager)
    }

    pub fn get_config(&self) -> &Config {
        &self.config
    }

    fn save_config(&self) -> Result<()> {
        self.ignore_next_change.store(true, Ordering::SeqCst);
        if let Err(e) = self.config.save(&self.config_path) {
            self.ignore_next_change.store(false, Ordering::SeqCst);
            return Err(e);
        }
        Ok(())
    }

    pub fn try_reload(&mut self) -> Option<Config> {
        let mut old_config = None;
        if let Ok(new_config) = Config::load(&self.config_path)
            && new_config != self.config
        {
            old_config = Some(mem::replace(&mut self.config, new_config));
        }
        self.save_config().ok();

        old_config
    }

    pub fn set_current_character(&mut self, character: String) -> Result<()> {
        self.config.current_character = character;
        self.save_config()
    }

    pub fn set_process_mode(&mut self, mode: ProcessMode) -> Result<()> {
        self.config.process_mode = mode;
        self.save_config()
    }

    pub fn set_intercept_enter(&mut self, enabled: bool) -> Result<()> {
        self.config.intercept_enter = enabled;
        self.save_config()
    }

    pub fn set_enable_whitelist(&mut self, enabled: bool) -> Result<()> {
        self.config.enable_whitelist = enabled;
        self.save_config()
    }
}
