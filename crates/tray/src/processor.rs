use std::thread;
use std::time::Duration;

use arboard::{Clipboard, ImageData};
use rdev::{EventType, Key, simulate};

use imagebox_core::{DataManager, generate_image};

use crate::config::{Config, ProcessMode};

fn send_key(event_type: &EventType) {
    simulate(event_type).ok();
    thread::sleep(Duration::from_millis(5));
}

fn simulate_key_combo(key: Key) {
    let modifier = if cfg!(target_os = "macos") {
        Key::MetaLeft
    } else {
        Key::ControlLeft
    };

    send_key(&EventType::KeyPress(modifier));
    send_key(&EventType::KeyPress(key));
    send_key(&EventType::KeyRelease(key));
    send_key(&EventType::KeyRelease(modifier));
}

pub fn process_image(
    config: &Config,
    data_manager: &DataManager,
    mode: ProcessMode,
    enable_max_chars: bool,
) {
    let Ok(mut clipboard) = Clipboard::new() else {
        return;
    };

    simulate_key_combo(Key::KeyA);
    thread::sleep(Duration::from_millis(20));

    simulate_key_combo(Key::KeyC);
    thread::sleep(Duration::from_millis(30));

    let Ok(copied_content) = clipboard.get_text() else {
        return;
    };

    if copied_content.is_empty() {
        return;
    }

    if mode == ProcessMode::Send
        && enable_max_chars
        && config.max_chars > 0
        && copied_content.chars().count() > config.max_chars
    {
        send_key(&EventType::KeyPress(Key::Return));
        send_key(&EventType::KeyRelease(Key::Return));
        return;
    }

    let image = {
        match generate_image(
            data_manager,
            &config.current_character,
            &copied_content,
            config.max_image_size,
            None,
        ) {
            Ok(img) => img,
            Err(_) => {
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

    if clipboard.set_image(image_data).is_err() {
        return;
    }

    if mode != ProcessMode::Copy {
        simulate_key_combo(Key::KeyV);
        thread::sleep(Duration::from_millis(100));

        if mode == ProcessMode::Send {
            send_key(&EventType::KeyPress(Key::Return));
            send_key(&EventType::KeyRelease(Key::Return));
        }
    }
}
