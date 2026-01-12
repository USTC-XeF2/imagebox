use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

use active_win_pos_rs::get_active_window;
use anyhow::Result;
use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::HotKey;
use rdev::{Event, EventType, Key, grab};

use crate::config::{Config, ConfigManager};

const SHIFT_MASK: u8 = 0b001;
const CTRL_MASK: u8 = 0b010;
const ALT_MASK: u8 = 0b100;

static MODIFIER_KEYS: AtomicU8 = AtomicU8::new(0);

fn set_modifier_key(mask: u8, pressed: bool) {
    if pressed {
        MODIFIER_KEYS.fetch_or(mask, Ordering::Relaxed);
    } else {
        MODIFIER_KEYS.fetch_and(!mask, Ordering::Relaxed);
    }
}

pub fn check_whitelist(config: &Config) -> bool {
    if !config.enable_whitelist {
        return true;
    }

    match get_active_window() {
        Ok(active_window) => config.whitelist.contains(&active_window.app_name),
        Err(()) => false,
    }
}

pub struct HotkeyManager {
    manager: GlobalHotKeyManager,

    pub toggle_hotkey: HotKey,
    pub generate_hotkey: HotKey,
}

impl HotkeyManager {
    pub fn new(config: &Config) -> Result<Self> {
        let manager = GlobalHotKeyManager::new()?;
        let toggle_hotkey = config.toggle_hotkey;
        let generate_hotkey = config.generate_hotkey;

        manager.register(toggle_hotkey).ok();
        manager.register(generate_hotkey).ok();

        Ok(Self {
            manager,
            toggle_hotkey,
            generate_hotkey,
        })
    }

    pub fn update(&mut self, config: &Config) {
        self.manager.unregister(self.toggle_hotkey).ok();
        self.manager.unregister(self.generate_hotkey).ok();

        self.toggle_hotkey = config.toggle_hotkey;
        self.generate_hotkey = config.generate_hotkey;

        self.manager.register(self.toggle_hotkey).ok();
        self.manager.register(self.generate_hotkey).ok();
    }
}

pub fn start_keyboard_listener<F>(
    config_manager: Arc<RwLock<ConfigManager>>,
    is_processing: Arc<Mutex<bool>>,
    on_enter: F,
) -> thread::JoinHandle<()>
where
    F: Fn() + Send + 'static,
{
    thread::spawn(move || {
        if let Err(error) =
            grab(move |event| handle_enter_key(event, &is_processing, &config_manager, &on_enter))
        {
            eprintln!("Error listening for events: {:?}", error);
        }
    })
}

fn handle_enter_key<F>(
    event: Event,
    is_processing: &Mutex<bool>,
    config_manager: &RwLock<ConfigManager>,
    on_enter: &F,
) -> Option<Event>
where
    F: Fn(),
{
    match event.event_type {
        EventType::KeyPress(Key::ShiftLeft | Key::ShiftRight) => {
            set_modifier_key(SHIFT_MASK, true);
        }
        EventType::KeyRelease(Key::ShiftLeft | Key::ShiftRight) => {
            set_modifier_key(SHIFT_MASK, false);
        }
        EventType::KeyPress(Key::ControlLeft | Key::ControlRight) => {
            set_modifier_key(CTRL_MASK, true);
        }
        EventType::KeyRelease(Key::ControlLeft | Key::ControlRight) => {
            set_modifier_key(CTRL_MASK, false);
        }
        EventType::KeyPress(Key::Alt | Key::AltGr) => {
            set_modifier_key(ALT_MASK, true);
        }
        EventType::KeyRelease(Key::Alt | Key::AltGr) => {
            set_modifier_key(ALT_MASK, false);
        }

        EventType::KeyPress(Key::Return) => {
            if MODIFIER_KEYS.load(Ordering::Relaxed) != 0 {
                return Some(event);
            }

            {
                let config_manager_guard = config_manager.read().unwrap();
                let config = config_manager_guard.get_config();
                if !config.intercept_enter || !check_whitelist(config) {
                    return Some(event);
                }
            };

            let processing = is_processing.lock().unwrap();
            if *processing {
                drop(processing);
                return Some(event);
            }
            drop(processing);

            on_enter();
            return None;
        }
        _ => {}
    }

    Some(event)
}
