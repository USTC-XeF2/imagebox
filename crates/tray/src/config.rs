use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

use anyhow::Result;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use notify_debouncer_full::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};
use rfd::MessageDialog;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub current_character: String,
    #[serde(default = "default_true")]
    pub auto_paste: bool,
    #[serde(default = "default_false")]
    pub auto_send: bool,
    #[serde(default = "default_false")]
    pub intercept_enter: bool,
    #[serde(default = "default_true")]
    pub enable_whitelist: bool,
    #[serde(default = "default_whitelist")]
    pub whitelist: Vec<String>,
    #[serde(default = "default_max_image_size")]
    pub max_image_size: usize,
    #[serde(default = "default_toggle_hotkey")]
    pub toggle_hotkey: HotKey,
    #[serde(default = "default_generate_hotkey")]
    pub generate_hotkey: HotKey,
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
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
            auto_paste: true,
            auto_send: false,
            intercept_enter: false,
            enable_whitelist: true,
            whitelist: default_whitelist(),
            max_image_size: default_max_image_size(),
            toggle_hotkey: default_toggle_hotkey(),
            generate_hotkey: default_generate_hotkey(),
        }
    }
}

pub fn get_current_dir() -> PathBuf {
    #[cfg(debug_assertions)]
    {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }

    #[cfg(not(debug_assertions))]
    {
        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

fn show_first_launch_guide() {
    MessageDialog::new()
        .set_title("欢迎使用 ImageBox")
        .set_description("感谢您使用 ImageBox！\n\n请通过系统托盘图标访问控制菜单。")
        .show();
}

impl Config {
    fn get_config_path() -> PathBuf {
        get_current_dir().join("config.yaml")
    }

    pub fn load() -> Self {
        let config_path = Self::get_config_path();
        let is_first_launch = !config_path.exists();

        if config_path.exists()
            && let Ok(content) = fs::read_to_string(&config_path)
            && let Ok(config) = serde_yaml::from_str(&content)
        {
            let loaded_config: Config = config;
            loaded_config.save().ok();
            return loaded_config;
        }

        let default_config = Config::default();
        default_config.save().ok();

        if is_first_launch {
            show_first_launch_guide();
        }

        default_config
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path();

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let yaml = serde_yaml::to_string(self)?;
        fs::write(config_path, yaml)?;
        Ok(())
    }

    pub fn set_current_character(&mut self, character: String) -> Result<()> {
        self.current_character = character;
        self.save()
    }

    pub fn set_auto_paste(&mut self, enabled: bool) -> Result<()> {
        self.auto_paste = enabled;
        self.save()
    }

    pub fn set_auto_send(&mut self, enabled: bool) -> Result<()> {
        self.auto_send = enabled;
        self.save()
    }

    pub fn set_intercept_enter(&mut self, enabled: bool) -> Result<()> {
        self.intercept_enter = enabled;
        self.save()
    }

    pub fn set_enable_whitelist(&mut self, enabled: bool) -> Result<()> {
        self.enable_whitelist = enabled;
        self.save()
    }
}

pub fn start_config_watcher(
    config_reload_sender: Sender<()>,
) -> Result<Debouncer<RecommendedWatcher, RecommendedCache>> {
    let mut debouncer = new_debouncer(
        Duration::from_millis(500),
        None,
        move |result: DebounceEventResult| {
            if let Ok(events) = result {
                for event in events {
                    for path in &event.paths {
                        if path.ends_with("config.yaml") {
                            config_reload_sender.send(()).ok();
                            break;
                        }
                    }
                }
            }
        },
    )?;

    debouncer.watch(get_current_dir(), RecursiveMode::NonRecursive)?;

    Ok(debouncer)
}
