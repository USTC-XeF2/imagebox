#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod keyboard;
mod processor;
mod tray;

use std::path::PathBuf;

use anyhow::Result;
use single_instance::SingleInstance;
use winit::event_loop::EventLoop;

use app::{App, UserEvent};

const APP_NAME: &str = "ImageBox_Tray_001";

pub fn get_current_dir() -> PathBuf {
    #[cfg(debug_assertions)]
    {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }

    #[cfg(not(debug_assertions))]
    {
        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

fn main() -> Result<()> {
    let instance = SingleInstance::new(APP_NAME)?;
    if instance.is_single() {
        let current_dir = get_current_dir();

        let event_loop = EventLoop::<UserEvent>::with_user_event().build().unwrap();
        let mut app = App::new(&current_dir, &event_loop)?;

        event_loop.run_app(&mut app)?;
    }

    Ok(())
}
