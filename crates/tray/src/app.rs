use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

use anyhow::{Error, Result};
use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
use rfd::{MessageDialog, MessageLevel};
use tray_icon::menu::MenuEvent;
use winit::application::ApplicationHandler;
use winit::event_loop::{ActiveEventLoop, ControlFlow};

use imagebox_core::DataManager;

use crate::config::{ConfigManager, ProcessMode};
use crate::keyboard::{HotkeyManager, check_whitelist, start_keyboard_listener};
use crate::processor::process_image;
use crate::tray::{ControlMessage, TrayMenu, create_tray_menu};

pub struct App {
    data_manager: Arc<DataManager>,
    is_processing: Arc<Mutex<bool>>,
    enter_key_receiver: Receiver<()>,
    tray_menu: TrayMenu,
    hotkey_manager: HotkeyManager,
    config_manager: Arc<RwLock<ConfigManager>>,
}

impl App {
    pub fn new(work_dir: &Path) -> Result<Self> {
        let show_resource_error = |e: Error| {
            MessageDialog::new()
                .set_level(MessageLevel::Error)
                .set_title("资源加载失败")
                .set_description(format!("{}", e))
                .show();
            e
        };

        let data_config_path = work_dir.join("data/data.json");
        let data_manager = DataManager::new(&data_config_path).map_err(show_resource_error)?;
        let characters = data_manager
            .get_characters()
            .iter()
            .map(|c| (c.id.clone(), c.name.clone()))
            .collect::<HashMap<_, _>>();

        let config_path = work_dir.join("config.yaml");
        let is_first_launch = !config_path.exists();

        let mut config_manager = ConfigManager::new(config_path)?;

        if is_first_launch {
            MessageDialog::new()
                .set_title("欢迎使用 ImageBox")
                .set_description("感谢您使用 ImageBox！\n\n请通过系统托盘图标访问控制菜单。")
                .show();
        }

        if !characters.contains_key(&config_manager.get_config().current_character) {
            config_manager
                .set_current_character(characters.keys().next().unwrap().clone())
                .ok();
        }

        let config = config_manager.get_config();

        let tray_menu = create_tray_menu(&characters, config)?;

        let hotkey_manager = HotkeyManager::new(config)?;

        let config_manager = Arc::new(RwLock::new(config_manager));
        let is_processing = Arc::new(Mutex::new(false));

        let (enter_key_sender, enter_key_receiver) = channel();
        start_keyboard_listener(
            config_manager.clone(),
            is_processing.clone(),
            enter_key_sender,
        );

        let data_manager = Arc::new(data_manager);

        Ok(Self {
            data_manager,
            is_processing,
            enter_key_receiver,
            tray_menu,
            hotkey_manager,
            config_manager,
        })
    }

    fn handle_reload_config(&mut self) {
        let config_manager = self.config_manager.read().unwrap();
        let new_config = config_manager.get_config();

        self.hotkey_manager.update(new_config);

        self.tray_menu.set_process_mode(new_config.process_mode);
        self.tray_menu
            .set_intercept_enter(new_config.intercept_enter);
        self.tray_menu
            .set_whitelist_enabled(new_config.enable_whitelist);

        let current_character = new_config.current_character.clone();
        drop(config_manager);

        if let Some(character_data) = self.data_manager.get_character(&current_character) {
            let character_name = character_data.name.clone();
            self.tray_menu.update_tooltip(&character_name);
            self.tray_menu.set_selected_character(&current_character);
        }
    }

    fn handle_hotkey_event(&mut self, event: GlobalHotKeyEvent, event_loop: &ActiveEventLoop) {
        if event.state == HotKeyState::Released {
            return;
        }

        if event.id == self.hotkey_manager.toggle_hotkey.id() {
            self.handle_message(ControlMessage::ToggleIntercept, event_loop);
        } else if event.id == self.hotkey_manager.generate_hotkey.id() {
            let (should_process, process_mode) = {
                let config_manager = self.config_manager.read().unwrap();
                let config = config_manager.get_config();
                (check_whitelist(config), config.process_mode)
            };

            if should_process {
                self.process_image_in_thread(process_mode, false);
            }
        }
    }

    fn handle_message(&mut self, msg: ControlMessage, event_loop: &ActiveEventLoop) {
        match msg {
            ControlMessage::SwitchCharacter(id) => {
                if let Some(character_data) = self.data_manager.get_character(&id) {
                    self.tray_menu.update_tooltip(&character_data.name);
                    self.tray_menu.set_selected_character(&id);

                    let mut config_manager = self.config_manager.write().unwrap();
                    config_manager.set_current_character(id.to_string()).ok();
                }
            }
            ControlMessage::ToggleAutoPaste => {
                let mut config_manager = self.config_manager.write().unwrap();
                let new_mode = match config_manager.get_config().process_mode {
                    ProcessMode::Copy => ProcessMode::Paste,
                    _ => ProcessMode::Copy,
                };
                config_manager.set_process_mode(new_mode).ok();

                self.tray_menu.set_process_mode(new_mode);
            }
            ControlMessage::ToggleAutoSend => {
                let mut config_manager = self.config_manager.write().unwrap();
                let new_mode = match config_manager.get_config().process_mode {
                    ProcessMode::Send => ProcessMode::Paste,
                    _ => ProcessMode::Send,
                };
                config_manager.set_process_mode(new_mode).ok();

                self.tray_menu.set_process_mode(new_mode);
            }
            ControlMessage::ToggleIntercept => {
                let mut config_manager = self.config_manager.write().unwrap();
                let new_enabled = !config_manager.get_config().intercept_enter;
                config_manager.set_intercept_enter(new_enabled).ok();

                self.tray_menu.set_intercept_enter(new_enabled);
            }
            ControlMessage::ToggleWhitelist => {
                let mut config_manager = self.config_manager.write().unwrap();
                let new_enabled = !config_manager.get_config().enable_whitelist;
                config_manager.set_enable_whitelist(new_enabled).ok();

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

    fn process_image_in_thread(&self, process_mode: ProcessMode, enable_max_chars: bool) {
        let mut processing = self.is_processing.lock().unwrap();
        if *processing {
            return;
        }
        *processing = true;

        let is_processing_clone = self.is_processing.clone();
        let data_manager = self.data_manager.clone();
        let config = self.config_manager.read().unwrap().get_config().clone();

        drop(processing);

        thread::spawn(move || {
            process_image(&config, &data_manager, process_mode, enable_max_chars);

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

        if self.config_manager.write().unwrap().try_reload() {
            self.handle_reload_config();
        }

        let tray_event_receiver = MenuEvent::receiver();
        if let Ok(event) = tray_event_receiver.try_recv()
            && let Some(msg) = self.tray_menu.event_to_message(&event.id)
        {
            self.handle_message(msg, event_loop);
        }

        let hotkey_receiver = GlobalHotKeyEvent::receiver();
        if let Ok(event) = hotkey_receiver.try_recv() {
            self.handle_hotkey_event(event, event_loop);
        }

        if self.enter_key_receiver.try_recv().is_ok() {
            self.process_image_in_thread(ProcessMode::Send, true);
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
