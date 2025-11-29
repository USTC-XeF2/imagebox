use std::collections::HashMap;

use anyhow::Result;
use tray_icon::menu::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use imagebox_core::CharacterData;

use crate::config::{Config, ProcessMode};

const ICON_DATA: &[u8] = include_bytes!("../assets/tray.raw");

pub enum ControlMessage {
    SwitchCharacter(String),
    ToggleAutoPaste,
    ToggleAutoSend,
    ToggleIntercept,
    ToggleWhitelist,
    Help,
    Quit,
}

pub struct TrayMenu {
    character_items: HashMap<String, CheckMenuItem>,
    character_id_map: HashMap<MenuId, String>,

    auto_paste_item: CheckMenuItem,
    auto_send_item: CheckMenuItem,

    intercept_item: CheckMenuItem,
    whitelist_item: CheckMenuItem,

    help_item: MenuItem,
    quit_item: MenuItem,

    tray_icon: TrayIcon,
    color_icon: Icon,
    gray_icon: Icon,
}

impl TrayMenu {
    pub fn update_tooltip(&self, character_name: &str) {
        let tooltip = format!("ImageBox - {}", character_name);
        self.tray_icon.set_tooltip(Some(tooltip)).ok();
    }

    pub fn set_intercept_enter(&self, enabled: bool) {
        self.intercept_item.set_checked(enabled);

        let icon = if enabled {
            self.color_icon.clone()
        } else {
            self.gray_icon.clone()
        };
        self.tray_icon.set_icon(Some(icon)).ok();
    }

    pub fn set_whitelist_enabled(&self, enabled: bool) {
        self.whitelist_item.set_checked(enabled);
    }

    pub fn set_process_mode(&self, mode: ProcessMode) {
        self.auto_paste_item.set_checked(mode != ProcessMode::Copy);
        self.auto_send_item.set_checked(mode == ProcessMode::Send);
    }

    pub fn set_selected_character(&self, character_id: &str) {
        for (id, menu_item) in &self.character_items {
            menu_item.set_checked(id == character_id);
        }
    }

    pub fn event_to_message(&self, event_id: &MenuId) -> Option<ControlMessage> {
        if event_id == self.auto_paste_item.id() {
            Some(ControlMessage::ToggleAutoPaste)
        } else if event_id == self.auto_send_item.id() {
            Some(ControlMessage::ToggleAutoSend)
        } else if event_id == self.intercept_item.id() {
            Some(ControlMessage::ToggleIntercept)
        } else if event_id == self.whitelist_item.id() {
            Some(ControlMessage::ToggleWhitelist)
        } else if event_id == self.help_item.id() {
            Some(ControlMessage::Help)
        } else if event_id == self.quit_item.id() {
            Some(ControlMessage::Quit)
        } else {
            self.character_id_map
                .get(event_id)
                .map(|name| ControlMessage::SwitchCharacter(name.clone()))
        }
    }
}

pub fn create_tray_menu(
    character_configs: &HashMap<String, CharacterData>,
    config: &Config,
) -> Result<TrayMenu> {
    let menu = Menu::new();

    let mut character_items = HashMap::new();
    let mut character_id_map = HashMap::new();

    let mut character_ids: Vec<_> = character_configs.keys().collect();
    character_ids.sort_unstable();

    for character_id in character_ids {
        if let Some(character_data) = character_configs.get(character_id) {
            let is_current = character_id == &config.current_character;
            let display_name = format!("{}({})", character_data.name, character_id);
            let item = CheckMenuItem::new(display_name, true, is_current, None);
            character_items.insert(character_id.clone(), item.clone());
            character_id_map.insert(item.id().clone(), character_id.clone());
            menu.append(&item)?;
        }
    }

    menu.append(&PredefinedMenuItem::separator())?;

    let auto_paste_item = CheckMenuItem::new(
        "自动粘贴",
        true,
        config.process_mode != ProcessMode::Copy,
        None,
    );
    menu.append(&auto_paste_item)?;

    let auto_send_item = CheckMenuItem::new(
        "自动发送",
        true,
        config.process_mode == ProcessMode::Send,
        None,
    );
    menu.append(&auto_send_item)?;

    menu.append(&PredefinedMenuItem::separator())?;

    let intercept_item = CheckMenuItem::new("启用 Enter 拦截", true, config.intercept_enter, None);
    menu.append(&intercept_item)?;

    let whitelist_item = CheckMenuItem::new("使用白名单", true, config.enable_whitelist, None);
    menu.append(&whitelist_item)?;

    menu.append(&PredefinedMenuItem::separator())?;

    let help_item = MenuItem::new("帮助", true, None);
    menu.append(&help_item)?;

    let quit_item = MenuItem::new("退出", true, None);
    menu.append(&quit_item)?;

    let color_icon = create_icon(false)?;
    let gray_icon = create_icon(true)?;

    let icon = if config.intercept_enter {
        color_icon.clone()
    } else {
        gray_icon.clone()
    };

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip(match character_configs.get(&config.current_character) {
            Some(CharacterData { name, .. }) => {
                format!("ImageBox - {}", name)
            }
            None => "ImageBox".to_string(),
        })
        .with_icon(icon)
        .build()?;

    let tray_menu = TrayMenu {
        character_items,
        character_id_map,
        auto_paste_item,
        auto_send_item,
        intercept_item,
        whitelist_item,
        help_item,
        quit_item,
        tray_icon,
        color_icon,
        gray_icon,
    };

    Ok(tray_menu)
}

fn create_icon(grayscale: bool) -> Result<Icon> {
    let mut rgba = ICON_DATA.to_vec();

    if grayscale {
        for chunk in rgba.chunks_exact_mut(4) {
            let r = chunk[0] as f32;
            let g = chunk[1] as f32;
            let b = chunk[2] as f32;
            let gray = (0.299 * r + 0.587 * g + 0.114 * b) as u8;
            chunk[0] = gray;
            chunk[1] = gray;
            chunk[2] = gray;
        }
    }

    Icon::from_rgba(rgba, 32, 32).map_err(Into::into)
}
