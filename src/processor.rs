use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use arboard::{Clipboard, ImageData};
use enigo::{Direction, Enigo, Key as EnigoKey, Keyboard, Settings};

use crate::config::Config;
use crate::data_manager::DataManager;
use crate::image_processor::generate_image;

fn simulate_key_combo(enigo: &mut Enigo, key: char) {
    enigo.key(EnigoKey::Control, Direction::Press).ok();
    thread::sleep(Duration::from_millis(5));
    enigo.key(EnigoKey::Unicode(key), Direction::Click).ok();
    thread::sleep(Duration::from_millis(5));
    enigo.key(EnigoKey::Control, Direction::Release).ok();
}

pub fn process_image(
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

    let (current_character, max_image_size) = {
        let config_guard = config.read().unwrap();
        (
            config_guard.current_character.clone(),
            config_guard.max_image_size,
        )
    };

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

    if paste {
        simulate_key_combo(&mut enigo, 'v');
        thread::sleep(Duration::from_millis(100));

        if send {
            enigo.key(EnigoKey::Return, Direction::Click).ok();
        }
    }
}
