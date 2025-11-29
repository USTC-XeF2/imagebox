use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use arboard::{Clipboard, ImageData};
use enigo::{Direction, Enigo, Key as EnigoKey, Keyboard, Settings};

use imagebox_core::{DataManager, generate_image};

use crate::config::{Config, ProcessMode};

fn simulate_key_combo(enigo: &mut Enigo, key: char) {
    enigo.key(EnigoKey::Control, Direction::Press).ok();
    thread::sleep(Duration::from_millis(5));
    enigo.key(EnigoKey::Unicode(key), Direction::Click).ok();
    thread::sleep(Duration::from_millis(5));
    enigo.key(EnigoKey::Control, Direction::Release).ok();
}

pub fn process_image(
    config: &Arc<RwLock<Config>>,
    data_manager: &Arc<RwLock<DataManager>>,
    mode: ProcessMode,
    enable_max_chars: bool,
) {
    let Ok(mut clipboard) = Clipboard::new() else {
        return;
    };

    let Ok(mut enigo) = Enigo::new(&Settings::default()) else {
        return;
    };

    simulate_key_combo(&mut enigo, 'a');
    thread::sleep(Duration::from_millis(20));

    simulate_key_combo(&mut enigo, 'c');
    thread::sleep(Duration::from_millis(30));

    let Ok(copied_content) = clipboard.get_text() else {
        return;
    };

    if copied_content.is_empty() {
        return;
    }

    let (current_character, max_image_size, max_chars) = {
        let config_guard = config.read().unwrap();
        (
            config_guard.current_character.clone(),
            config_guard.max_image_size,
            config_guard.max_chars,
        )
    };

    if mode == ProcessMode::Send
        && enable_max_chars
        && max_chars > 0
        && copied_content.chars().count() > max_chars
    {
        enigo.key(EnigoKey::Return, Direction::Click).ok();
        return;
    }

    let image = {
        let data_manager_guard = data_manager.read().unwrap();
        match generate_image(
            &data_manager_guard,
            &current_character,
            &copied_content,
            max_image_size,
        ) {
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

    if clipboard.set_image(image_data).is_err() {
        return;
    }

    if mode != ProcessMode::Copy {
        simulate_key_combo(&mut enigo, 'v');
        thread::sleep(Duration::from_millis(100));

        if mode == ProcessMode::Send {
            enigo.key(EnigoKey::Return, Direction::Click).ok();
        }
    }
}
