use active_win_pos_rs::get_active_window;
use arboard::{Clipboard, ImageData};
use enigo::{Direction, Enigo, Key as EnigoKey, Keyboard, Settings};
use rdev::{Event, EventType, Key, grab};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

use crate::config::Config;
use crate::data_manager::DataManager;
use crate::image_processor::generate_image;
use crate::tray::ControlMessage;

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

fn check_whitelist(config: &Arc<RwLock<Config>>) -> bool {
    let enable_whitelist = {
        let config_guard = config.read().unwrap();
        config_guard.enable_whitelist
    };

    if !enable_whitelist {
        return true;
    }

    match get_active_window() {
        Ok(active_window) => {
            let config_guard = config.read().unwrap();
            config_guard.whitelist.contains(&active_window.app_name)
        }
        Err(_) => false,
    }
}

pub fn start_keyboard_listener(
    config: Arc<RwLock<Config>>,
    data_manager: Arc<RwLock<DataManager>>,
    message_sender: Sender<ControlMessage>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let is_processing = Arc::new(Mutex::new(false));

        if let Err(error) = grab(move |event| {
            handle_keyboard_event(
                event,
                is_processing.clone(),
                config.clone(),
                data_manager.clone(),
                message_sender.clone(),
            )
        }) {
            eprintln!("Error listening for events: {:?}", error);
        }
    })
}

fn handle_keyboard_event(
    event: Event,
    is_processing: Arc<Mutex<bool>>,
    config: Arc<RwLock<Config>>,
    data_manager: Arc<RwLock<DataManager>>,
    message_sender: Sender<ControlMessage>,
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

        EventType::KeyPress(Key::KeyT) => {
            if MODIFIER_KEYS.load(Ordering::Relaxed) == (CTRL_MASK | ALT_MASK) {
                let _ = message_sender.send(ControlMessage::ToggleIntercept);
                return None;
            }
        }

        EventType::KeyPress(Key::KeyE) => {
            if MODIFIER_KEYS.load(Ordering::Relaxed) == CTRL_MASK {
                if !check_whitelist(&config) {
                    return Some(event);
                }

                let (auto_paste, auto_send) = {
                    let config_guard = config.read().unwrap();
                    (config_guard.auto_paste, config_guard.auto_send)
                };

                if process_image_in_thread(
                    is_processing,
                    config,
                    data_manager,
                    auto_paste,
                    auto_send,
                ) {
                    return None;
                };
            }
        }

        EventType::KeyPress(Key::Return) => {
            if MODIFIER_KEYS.load(Ordering::Relaxed) != 0 {
                return Some(event);
            }

            let intercept_enter = {
                let config_guard = config.read().unwrap();
                config_guard.intercept_enter
            };

            if !intercept_enter || !check_whitelist(&config) {
                return Some(event);
            }

            if process_image_in_thread(is_processing, config, data_manager, true, true) {
                return None;
            };
        }

        _ => {}
    }

    Some(event)
}

fn process_image_in_thread(
    is_processing: Arc<Mutex<bool>>,
    config: Arc<RwLock<Config>>,
    data_manager: Arc<RwLock<DataManager>>,
    auto_paste: bool,
    auto_send: bool,
) -> bool {
    let mut processing = is_processing.lock().unwrap();
    if *processing {
        return false;
    }
    *processing = true;

    let is_processing_clone = is_processing.clone();
    let config_clone = config.clone();
    let data_manager_clone = data_manager.clone();

    drop(processing);

    thread::spawn(move || {
        process_image(config_clone, data_manager_clone, auto_paste, auto_send);

        if let Ok(mut processing) = is_processing_clone.lock() {
            *processing = false;
        }
    });

    true
}

fn simulate_key_combo(enigo: &mut Enigo, key: char) {
    enigo.key(EnigoKey::Control, Direction::Press).ok();
    thread::sleep(Duration::from_millis(5));
    enigo.key(EnigoKey::Unicode(key), Direction::Click).ok();
    thread::sleep(Duration::from_millis(5));
    enigo.key(EnigoKey::Control, Direction::Release).ok();
}

fn process_image(
    config: Arc<RwLock<Config>>,
    data_manager: Arc<RwLock<DataManager>>,
    paste: bool,
    send: bool,
) {
    let mut clipboard = match Clipboard::new() {
        Ok(cb) => cb,
        Err(_) => {
            return;
        }
    };

    let mut enigo = match Enigo::new(&Settings::default()) {
        Ok(e) => e,
        Err(_) => {
            return;
        }
    };

    simulate_key_combo(&mut enigo, 'a');
    thread::sleep(Duration::from_millis(20));

    simulate_key_combo(&mut enigo, 'c');
    thread::sleep(Duration::from_millis(30));

    let copied_content = match clipboard.get_text() {
        Ok(text) => text,
        Err(_) => {
            return;
        }
    };

    if copied_content.is_empty() {
        return;
    }

    let current_character = {
        let config_guard = config.read().unwrap();
        config_guard.current_character.clone()
    };

    let image = {
        let data_manager_guard = data_manager.read().unwrap();
        match generate_image(&data_manager_guard, &current_character, &copied_content) {
            Some(img) => img,
            None => {
                return;
            }
        }
    };

    let (width, height) = image.dimensions();
    let image_data = ImageData {
        width: width as usize,
        height: height as usize,
        bytes: image.into_raw().into(),
    };

    if let Err(_) = clipboard.set_image(image_data) {
        return;
    }

    if paste {
        simulate_key_combo(&mut enigo, 'v');
        thread::sleep(Duration::from_millis(100));

        if send {
            enigo.key(EnigoKey::Return, Direction::Click).ok();
        }
    }
}
