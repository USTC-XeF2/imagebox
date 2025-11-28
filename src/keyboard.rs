use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

use active_win_pos_rs::get_active_window;
use global_hotkey::GlobalHotKeyManager;
use global_hotkey::hotkey::HotKey;
use rdev::{Event, EventType, Key, grab};

use crate::config::Config;

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
        Err(_) => false,
    }
}

pub fn start_keyboard_listener(
    config: Arc<RwLock<Config>>,
    is_processing: Arc<Mutex<bool>>,
    enter_key_sender: Sender<()>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let config_clone = config.clone();
        let is_processing_clone = is_processing.clone();
        let enter_key_sender_clone = enter_key_sender.clone();

        thread::spawn(move || {
            if let Err(error) = grab(move |event| {
                handle_enter_key(
                    event,
                    is_processing_clone.clone(),
                    config_clone.clone(),
                    enter_key_sender_clone.clone(),
                )
            }) {
                eprintln!("Error listening for events: {:?}", error);
            }
        });
    })
}

pub struct HotkeyManager {
    manager: GlobalHotKeyManager,

    pub toggle_hotkey: HotKey,
    pub generate_hotkey: HotKey,
}

impl HotkeyManager {
    pub fn new(config: &Config) -> global_hotkey::Result<Self> {
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

fn handle_enter_key(
    event: Event,
    is_processing: Arc<Mutex<bool>>,
    config: Arc<RwLock<Config>>,
    enter_key_sender: Sender<()>,
) -> Option<Event> {
    match event.event_type {
        EventType::KeyPress(Key::ShiftLeft) | EventType::KeyPress(Key::ShiftRight) => {
            set_modifier_key(SHIFT_MASK, true);
        }
        EventType::KeyRelease(Key::ShiftLeft) | EventType::KeyRelease(Key::ShiftRight) => {
            set_modifier_key(SHIFT_MASK, false);
        }
        EventType::KeyPress(Key::ControlLeft) | EventType::KeyPress(Key::ControlRight) => {
            set_modifier_key(CTRL_MASK, true);
        }
        EventType::KeyRelease(Key::ControlLeft) | EventType::KeyRelease(Key::ControlRight) => {
            set_modifier_key(CTRL_MASK, false);
        }
        EventType::KeyPress(Key::Alt) | EventType::KeyPress(Key::AltGr) => {
            set_modifier_key(ALT_MASK, true);
        }
        EventType::KeyRelease(Key::Alt) | EventType::KeyRelease(Key::AltGr) => {
            set_modifier_key(ALT_MASK, false);
        }

        EventType::KeyPress(Key::Return) => {
            if MODIFIER_KEYS.load(Ordering::Relaxed) != 0 {
                return Some(event);
            }

            {
                let config_guard = config.read().unwrap();
                if !config_guard.intercept_enter || !check_whitelist(&config_guard) {
                    return Some(event);
                }
            };

            let processing = is_processing.lock().unwrap();
            if *processing {
                drop(processing);
                return Some(event);
            }
            drop(processing);

            let _ = enter_key_sender.send(());
            return None;
        }
        _ => {}
    }

    Some(event)
}
