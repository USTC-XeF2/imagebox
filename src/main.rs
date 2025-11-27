#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod data_manager;
mod image_processor;
mod keyboard;
mod loader;
mod tray;

use single_instance::SingleInstance;
use std::sync::mpsc::channel;
use std::sync::{Arc, RwLock};
use winit::event_loop::EventLoop;
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;

use app::App;
use config::{Config, start_config_watcher};
use data_manager::DataManager;
use keyboard::start_keyboard_listener;
use tray::create_tray_menu;

const APP_NAME: &str = "ImageBox_001";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = SingleInstance::new(APP_NAME).unwrap();
    if !instance.is_single() {
        return Ok(());
    }

    let mut data_manager = DataManager::init();
    let characters = data_manager
        .character_configs
        .keys()
        .cloned()
        .collect::<Vec<String>>();

    let config = Config::load();
    let config = if !characters.contains(&config.current_character) {
        let mut new_config = config;
        new_config.set_current_character(characters[0].clone()).ok();
        new_config
    } else {
        config
    };

    data_manager.switch_to_character(&config.current_character);

    let tray_menu = create_tray_menu(&data_manager.character_configs, &config)?;

    let data_manager = Arc::new(RwLock::new(data_manager));
    let config = Arc::new(RwLock::new(config));

    let (message_sender, message_receiver) = channel();
    start_keyboard_listener(config.clone(), data_manager.clone(), message_sender);

    let (config_reload_sender, config_reload_receiver) = channel();
    let _config_watcher = start_config_watcher(config_reload_sender)?;

    let mut app = App {
        message_receiver,
        config_reload_receiver,
        tray_menu,
        config,
        data_manager,
    };

    let mut event_loop = EventLoop::new()?;
    event_loop.run_app_on_demand(&mut app)?;

    Ok(())
}
