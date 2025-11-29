#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod keyboard;
mod processor;
mod tray;

use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex, RwLock};

use anyhow::{Error, Result};
use rfd::{MessageDialog, MessageLevel};
use single_instance::SingleInstance;
use winit::event_loop::EventLoop;
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;

use imagebox_core::DataManager;

use app::App;
use config::{Config, start_config_watcher};
use keyboard::{HotkeyManager, start_keyboard_listener};
use tray::create_tray_menu;

use crate::config::get_current_dir;

const APP_NAME: &str = "ImageBox_001";

fn main() -> Result<()> {
    let instance = SingleInstance::new(APP_NAME).unwrap();
    if !instance.is_single() {
        return Ok(());
    }

    let show_resource_error = |e: Error| {
        MessageDialog::new()
            .set_level(MessageLevel::Error)
            .set_title("资源加载失败")
            .set_description(format!("{}", e))
            .show();
        e
    };

    let data_dir = get_current_dir().join("data");
    let mut data_manager = DataManager::new(data_dir).map_err(show_resource_error)?;

    let characters = data_manager
        .character_configs
        .keys()
        .cloned()
        .collect::<Vec<String>>();

    let config = Config::load();
    let config = if characters.contains(&config.current_character) {
        config
    } else {
        let mut new_config = config;
        new_config.set_current_character(characters[0].clone()).ok();
        new_config
    };

    data_manager
        .switch_to_character(&config.current_character)
        .map_err(show_resource_error)?;

    let tray_menu = create_tray_menu(&data_manager.character_configs, &config)?;

    let hotkey_manager = HotkeyManager::new(&config)?;

    let config = Arc::new(RwLock::new(config));
    let is_processing = Arc::new(Mutex::new(false));

    let (enter_key_sender, enter_key_receiver) = channel();
    start_keyboard_listener(config.clone(), is_processing.clone(), enter_key_sender);

    let (config_reload_sender, config_reload_receiver) = channel();
    let _config_watcher = start_config_watcher(config_reload_sender)?;

    let data_manager = Arc::new(RwLock::new(data_manager));

    let mut app = App {
        is_processing,
        enter_key_receiver,
        config_reload_receiver,
        tray_menu,
        hotkey_manager,
        config,
        data_manager,
    };

    let mut event_loop = EventLoop::new()?;
    event_loop.run_app_on_demand(&mut app)?;

    Ok(())
}
