use std::collections::HashMap;

use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::config::Config;
use crate::data_manager::CharacterData;

const ICON_DATA: &[u8] = include_bytes!("../assets/tray.png");

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

    menu_channel: &'static tray_icon::menu::MenuEventReceiver,
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

    pub fn set_auto_paste_enabled(&self, enabled: bool) {
        self.auto_paste_item.set_checked(enabled);
    }

    pub fn set_auto_send_enabled(&self, enabled: bool) {
        self.auto_send_item.set_checked(enabled);
    }

    pub fn set_selected_character(&self, character_name: &str) {
        for (name, menu_item) in &self.character_items {
            menu_item.set_checked(name == character_name);
        }
    }

    pub fn try_recv(&self) -> Option<ControlMessage> {
        if let Ok(event) = self.menu_channel.try_recv() {
            if event.id == self.auto_paste_item.id() {
                Some(ControlMessage::ToggleAutoPaste)
            } else if event.id == self.auto_send_item.id() {
                Some(ControlMessage::ToggleAutoSend)
            } else if event.id == self.intercept_item.id() {
                Some(ControlMessage::ToggleIntercept)
            } else if event.id == self.whitelist_item.id() {
                Some(ControlMessage::ToggleWhitelist)
            } else if event.id == self.help_item.id() {
                Some(ControlMessage::Help)
            } else if event.id == self.quit_item.id() {
                Some(ControlMessage::Quit)
            } else if let Some(name) = self.character_id_map.get(&event.id) {
                Some(ControlMessage::SwitchCharacter(name.clone()))
            } else {
                None
            }
        } else {
            None
        }
    }
}

pub fn create_tray_menu(
    character_configs: &HashMap<String, CharacterData>,
    config: &Config,
) -> Result<TrayMenu, Box<dyn std::error::Error>> {
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

    let auto_paste_item = CheckMenuItem::new("自动粘贴", true, config.auto_paste, None);
    menu.append(&auto_paste_item)?;

    let auto_send_item = CheckMenuItem::new("自动发送", true, config.auto_send, None);
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

    let menu_channel = MenuEvent::receiver();

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
        menu_channel,
    };

    Ok(tray_menu)
}

fn create_icon(grayscale: bool) -> Result<Icon, Box<dyn std::error::Error>> {
    let img = image::load_from_memory(ICON_DATA)?;
    let mut rgba = img.to_rgba8();

    if grayscale {
        for pixel in rgba.pixels_mut() {
            let r = pixel[0] as f32;
            let g = pixel[1] as f32;
            let b = pixel[2] as f32;
            let gray = (0.299 * r + 0.587 * g + 0.114 * b) as u8;
            pixel[0] = gray;
            pixel[1] = gray;
            pixel[2] = gray;
        }
    }

    let (width, height) = rgba.dimensions();
    Icon::from_rgba(rgba.into_raw(), width, height).map_err(|e| e.into())
}
