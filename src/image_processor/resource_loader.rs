use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use ab_glyph::FontVec;
use image::{ImageReader, RgbaImage};
use rand::Rng;

use crate::data_manager::get_data_dir;

pub fn load_image(path: &PathBuf) -> Option<RgbaImage> {
    match ImageReader::open(path) {
        Ok(reader) => match reader.decode() {
            Ok(img) => Some(img.to_rgba8()),
            Err(_) => None,
        },
        Err(_) => None,
    }
}

pub fn load_random_image<T: Rng>(rng: &mut T, paths: &[&PathBuf]) -> Option<RgbaImage> {
    if paths.is_empty() {
        return None;
    }

    for _ in 0..3 {
        let idx = rng.random_range(0..paths.len());
        if let Some(img) = load_image(paths[idx]) {
            return Some(img);
        }
    }

    None
}

pub fn load_font(font: &str) -> Option<Arc<FontVec>> {
    let font_path = get_data_dir().join("fonts").join(font);
    match fs::read(font_path) {
        Ok(font_data) => match FontVec::try_from_vec(font_data) {
            Ok(font) => Some(Arc::new(font)),
            Err(_) => None,
        },
        Err(_) => None,
    }
}
