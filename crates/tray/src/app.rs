use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use tray_icon::menu::MenuEvent;
use winit::application::ApplicationHandler;
use winit::event_loop::{ActiveEventLoop, ControlFlow};

use imagebox_core::DataManager;

use crate::config::Config;
use crate::keyboard::{HotkeyManager, check_whitelist};
use crate::processor::process_image;
use crate::tray::{ControlMessage, TrayMenu};

pub struct App {
    pub is_processing: Arc<Mutex<bool>>,
    pub enter_key_receiver: Receiver<()>,
    pub config_reload_receiver: Receiver<()>,
    pub tray_menu: TrayMenu,
    pub hotkey_manager: HotkeyManager,
    pub config: Arc<RwLock<Config>>,
    pub data_manager: Arc<RwLock<DataManager>>,
}

impl App {
    fn reload_config(&mut self) {
        let new_config = Config::load();

        self.hotkey_manager.update(&new_config);

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
            if let Some(character_data) = data_manager
                .character_configs
                .get(&new_config.current_character)
            {
                let mut data_manager = self.data_manager.write().unwrap();
                if data_manager
                    .switch_to_character(&new_config.current_character)
                    .is_err()
                {
                    return;
                }
                drop(data_manager);

                self.tray_menu.update_tooltip(&character_data.name);

                self.tray_menu
                    .set_selected_character(&new_config.current_character);
            }
        }
    }

    fn handle_tray_event(&mut self, event: MenuEvent, event_loop: &ActiveEventLoop) {
        if let Some(msg) = self.tray_menu.event_to_message(&event.id) {
            self.handle_message(msg, event_loop);
        }
    }

    fn handle_hotkey_event(
        &mut self,
        event: global_hotkey::GlobalHotKeyEvent,
        event_loop: &ActiveEventLoop,
    ) {
        if event.state == HotKeyState::Released {
            return;
        }

        if event.id == self.hotkey_manager.toggle_hotkey.id() {
            self.handle_message(ControlMessage::ToggleIntercept, event_loop);
        } else if event.id == self.hotkey_manager.generate_hotkey.id() {
            let (should_process, auto_paste, auto_send) = {
                let config_guard = self.config.read().unwrap();
                (
                    check_whitelist(&config_guard),
                    config_guard.auto_paste,
                    config_guard.auto_send,
                )
            };

            if should_process {
                self.process_image_in_thread(auto_paste, auto_send);
            }
        }
    }

    fn handle_message(&mut self, msg: ControlMessage, event_loop: &ActiveEventLoop) {
        match msg {
            ControlMessage::SwitchCharacter(name) => {
                let mut data_manager = self.data_manager.write().unwrap();
                if data_manager.switch_to_character(&name).is_err() {
                    return;
                }

                if let Some(character_data) = data_manager.character_configs.get(&name) {
                    self.tray_menu.update_tooltip(&character_data.name);
                }
                drop(data_manager);

                self.tray_menu.set_selected_character(&name);

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

    fn process_image_in_thread(&self, auto_paste: bool, auto_send: bool) {
        let mut processing = self.is_processing.lock().unwrap();
        if *processing {
            return;
        }
        *processing = true;

        let is_processing_clone = self.is_processing.clone();
        let config_clone = self.config.clone();
        let data_manager_clone = self.data_manager.clone();

        drop(processing);

        thread::spawn(move || {
            process_image(&config_clone, &data_manager_clone, auto_paste, auto_send);

            if let Ok(mut processing) = is_processing_clone.lock() {
                *processing = false;
            }
        });
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

        let tray_event_receiver = MenuEvent::receiver();
        if let Ok(event) = tray_event_receiver.try_recv() {
            self.handle_tray_event(event, event_loop);
        }

        let hotkey_receiver = GlobalHotKeyEvent::receiver();
        if let Ok(event) = hotkey_receiver.try_recv() {
            self.handle_hotkey_event(event, event_loop);
        }

        if self.enter_key_receiver.try_recv().is_ok() {
            self.process_image_in_thread(true, true);
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
