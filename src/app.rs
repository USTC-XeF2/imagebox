use std::sync::mpsc::Receiver;
use std::sync::{Arc, RwLock};

use winit::application::ApplicationHandler;
use winit::event_loop::{ActiveEventLoop, ControlFlow};

use crate::config::Config;
use crate::data_manager::DataManager;
use crate::tray::{ControlMessage, TrayMenu};

pub struct App {
    pub message_receiver: Receiver<ControlMessage>,
    pub config_reload_receiver: Receiver<()>,
    pub tray_menu: TrayMenu,
    pub config: Arc<RwLock<Config>>,
    pub data_manager: Arc<RwLock<DataManager>>,
}

impl App {
    fn reload_config(&mut self) {
        let new_config = Config::load();

        let (config_changed, character_changed) = {
            let config = self.config.read().unwrap();
            let config_changed = *config != new_config;
            let character_changed = config.current_character != new_config.current_character;
            (config_changed, character_changed)
        };

        if !config_changed {
            return;
        }

        {
            let mut config = self.config.write().unwrap();
            *config = new_config.clone();
        }

        self.tray_menu.set_auto_paste_enabled(new_config.auto_paste);
        self.tray_menu.set_auto_send_enabled(new_config.auto_send);
        self.tray_menu
            .set_intercept_enter(new_config.intercept_enter);
        self.tray_menu
            .set_whitelist_enabled(new_config.enable_whitelist);

        if character_changed {
            let data_manager = self.data_manager.read().unwrap();
            if data_manager
                .character_configs
                .contains_key(&new_config.current_character)
            {
                if let Some(character_data) = data_manager
                    .character_configs
                    .get(&new_config.current_character)
                {
                    self.tray_menu.update_tooltip(&character_data.name);
                }

                let mut data_manager = self.data_manager.write().unwrap();
                data_manager.switch_to_character(&new_config.current_character);
                drop(data_manager);

                self.tray_menu
                    .set_selected_character(&new_config.current_character);
            }
        }
    }

    fn handle_message(&mut self, msg: ControlMessage, event_loop: &ActiveEventLoop) {
        match msg {
            ControlMessage::SwitchCharacter(name) => {
                self.tray_menu.set_selected_character(&name);

                let data_manager = self.data_manager.read().unwrap();
                if let Some(character_data) = data_manager.character_configs.get(&name) {
                    self.tray_menu.update_tooltip(&character_data.name);
                }
                drop(data_manager);

                let mut data_manager = self.data_manager.write().unwrap();
                data_manager.switch_to_character(&name);

                let mut config = self.config.write().unwrap();
                config.set_current_character(name.to_string()).ok();
            }
            ControlMessage::ToggleAutoPaste => {
                let mut config = self.config.write().unwrap();
                let new_enabled = !config.auto_paste;
                config.set_auto_paste(new_enabled).ok();

                self.tray_menu.set_auto_paste_enabled(new_enabled);
            }
            ControlMessage::ToggleAutoSend => {
                let mut config = self.config.write().unwrap();
                let new_enabled = !config.auto_send;
                config.set_auto_send(new_enabled).ok();

                self.tray_menu.set_auto_send_enabled(new_enabled);
            }
            ControlMessage::ToggleIntercept => {
                let mut config = self.config.write().unwrap();
                let new_enabled = !config.intercept_enter;
                config.set_intercept_enter(new_enabled).ok();

                self.tray_menu.set_intercept_enter(new_enabled);
            }
            ControlMessage::ToggleWhitelist => {
                let mut config = self.config.write().unwrap();
                let new_enabled = !config.enable_whitelist;
                config.set_enable_whitelist(new_enabled).ok();

                self.tray_menu.set_whitelist_enabled(new_enabled);
            }
            ControlMessage::Help => {
                open::that("https://github.com/USTC-XeF2/imagebox").ok();
            }
            ControlMessage::Quit => {
                event_loop.exit();
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        _event: winit::event::WindowEvent,
    ) {
        event_loop.set_control_flow(ControlFlow::Poll);
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, _cause: winit::event::StartCause) {
        event_loop.set_control_flow(ControlFlow::Poll);

        if self.config_reload_receiver.try_recv().is_ok() {
            self.reload_config();
        }

        if let Some(msg) = self.tray_menu.try_recv() {
            self.handle_message(msg, event_loop);
        }

        if let Ok(msg) = self.message_receiver.try_recv() {
            self.handle_message(msg, event_loop);
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
