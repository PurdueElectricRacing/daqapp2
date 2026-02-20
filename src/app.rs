use crate::{can, config, shortcuts, ui, widgets, workspace};
use eframe::egui::{self};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub enum ThemeSelection {
    Default,
    Nord,
    Catppuccin,
}

pub struct DAQApp {
    pub is_sidebar_open: bool,
    pub tile_tree: egui_tiles::Tree<widgets::Widget>,
    pub next_can_viewer_num: usize,
    pub next_bootloader_num: usize,
    pub next_scope_num: usize,
    pub next_log_parser_num: usize,
    pub can_receiver: std::sync::mpsc::Receiver<can::can_messages::CanMessage>,
    pub ui_sender: std::sync::mpsc::Sender<ui::ui_messages::UiMessage>,
    pub pending_scope_spawns: Vec<(u32, String, String)>,
    pub theme: egui::Style,
    pub theme_selection: ThemeSelection,
    pub pixels_per_point: f32,
    pub selected_serial: Option<String>,
    pub serial_ports: Vec<serialport::SerialPortInfo>,
    pub dbc_path: Option<std::path::PathBuf>,
    pub connection_error: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub dbc_path: Option<std::path::PathBuf>,
    pub selected_serial: Option<String>,
    pub theme: ThemeSelection,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeSelection::Default,
            dbc_path: None,
            selected_serial: None,
        }
    }
}

impl Settings {
    pub fn load(path: &str) -> Self {
        if let Ok(json) = fs::read_to_string(path) {
            serde_json::from_str(&json).unwrap_or_default()
        } else {
            let default = Settings::default();
            default.save(path);
            default
        }
    }

    pub fn save(&self, path: &str) {
        let json = serde_json::to_string_pretty(self).expect("Failed to serialize settings");
        fs::write(path, json).expect("Failed to write settings.json");
    }
}

const MIN_UI_SCALE: f32 = 0.4;
const MAX_UI_SCALE: f32 = 5.0;

impl DAQApp {
    pub fn save_settings(&self) {
        let settings = Settings {
            dbc_path: self.dbc_path.clone(),
            selected_serial: self.selected_serial.clone(),
            theme: self.theme_selection,
        };
        settings.save("settings.json");
    }

    pub fn new(
        can_receiver: std::sync::mpsc::Receiver<can::can_messages::CanMessage>,
        ui_sender: std::sync::mpsc::Sender<ui::ui_messages::UiMessage>,
        cc: &eframe::CreationContext,
    ) -> Self {
        // Calculate a default ui scale based off the native_pixels_per_point
        let native_ppp = cc.egui_ctx.native_pixels_per_point().unwrap_or(1.0);
        let default_scale = (native_ppp * 2.4).clamp(MIN_UI_SCALE, MAX_UI_SCALE);
        let settings_path = "settings.json".to_string();
        let settings = Settings::load(&settings_path);
        let theme1 = settings.theme.clone();
        let theme_selection = settings.theme;
        let theme = match theme1 {
            ThemeSelection::Default => egui::Style::default(),
            ThemeSelection::Nord => config::ThemeColors::load_from_file("themes/nord.toml")
                .map(|t| t.to_egui_style())
                .unwrap_or_default(),
            ThemeSelection::Catppuccin => {
                config::ThemeColors::load_from_file("themes/catppuccin.toml")
                    .map(|t| t.to_egui_style())
                    .unwrap_or_default()
            }
        };

        let selected_serial = settings.selected_serial.clone();
        let dbc_path = settings.dbc_path.clone();
        Self {
            is_sidebar_open: true,
            tile_tree: egui_tiles::Tree::empty("workspace_tree"),
            next_can_viewer_num: 1,
            next_bootloader_num: 1,
            next_scope_num: 1,
            next_log_parser_num: 1,
            can_receiver,
            ui_sender,
            pending_scope_spawns: Vec::new(),
            theme,
            theme_selection,
            pixels_per_point: default_scale,
            serial_ports: Vec::new(),
            selected_serial,
            dbc_path,
            connection_error: None,
        }
    }

    fn add_widget_to_tree(&mut self, widget: widgets::Widget) {
        let new_tile_id = self.tile_tree.tiles.insert_pane(widget);

        // No root yet, this becomes the root
        let Some(root_id) = self.tile_tree.root else {
            self.tile_tree.root = Some(new_tile_id);
            return;
        };

        // Check if root is already a tab container
        let Some(egui_tiles::Tile::Container(egui_tiles::Container::Tabs(tabs))) =
            self.tile_tree.tiles.get_mut(root_id)
        else {
            // Root is not a tab container, create one
            let tab_container = self
                .tile_tree
                .tiles
                .insert_tab_tile(vec![root_id, new_tile_id]);
            self.tile_tree.root = Some(tab_container);
            return;
        };

        // Root is already a tab container, add to it
        tabs.add_child(new_tile_id);
        tabs.set_active(new_tile_id);
    }

    pub fn spawn_viewer_table(&mut self) {
        let widget = widgets::Widget::ViewerTable(ui::viewer_table::ViewerTable::new(
            self.next_can_viewer_num,
        ));
        self.next_can_viewer_num += 1;
        self.add_widget_to_tree(widget);
    }

    pub fn spawn_viewer_list(&mut self) {
        let widget =
            widgets::Widget::ViewerList(ui::viewer_list::ViewerList::new(self.next_can_viewer_num));
        self.next_can_viewer_num += 1;
        self.add_widget_to_tree(widget);
    }

    pub fn spawn_bootloader(&mut self) {
        let widget =
            widgets::Widget::Bootloader(ui::bootloader::Bootloader::new(self.next_bootloader_num));
        self.next_bootloader_num += 1;
        self.add_widget_to_tree(widget);
    }

    pub fn spawn_scope(&mut self, msg_id: u32, msg_name: String, signal_name: String) {
        let widget = widgets::Widget::Scope(ui::scope::Scope::new(
            self.next_scope_num,
            msg_id,
            msg_name,
            signal_name,
        ));
        self.next_scope_num += 1;
        self.add_widget_to_tree(widget);
    }

    pub fn spawn_log_parser(&mut self) {
        let widget =
            widgets::Widget::LogParser(ui::log_parser::LogParser::new(self.next_log_parser_num));
        self.next_log_parser_num += 1;
        self.add_widget_to_tree(widget);
    }

    pub fn toggle_theme(&mut self) {
        // Update theme selection to the next option
        self.theme_selection = match self.theme_selection {
            ThemeSelection::Default => ThemeSelection::Nord,
            ThemeSelection::Nord => ThemeSelection::Catppuccin,
            ThemeSelection::Catppuccin => ThemeSelection::Default,
        };

        // Load the selected theme into the actual field
        self.theme = match self.theme_selection {
            ThemeSelection::Default => egui::Style::default(),
            ThemeSelection::Nord => config::ThemeColors::load_from_file("themes/nord.toml")
                .map(|t| t.to_egui_style())
                .unwrap_or_default(),
            ThemeSelection::Catppuccin => {
                config::ThemeColors::load_from_file("themes/catppuccin.toml")
                    .map(|t| t.to_egui_style())
                    .unwrap_or_default()
            }
        };
    }

    // Close the currently active widget in the tile tree
    pub fn close_active_widget(&mut self) {
        let active_tiles = self.tile_tree.active_tiles();

        for tile_id in active_tiles {
            if let Some(egui_tiles::Tile::Pane(_)) = self.tile_tree.tiles.get(tile_id) {
                self.tile_tree.tiles.remove(tile_id);
                break;
            }
        }
    }
}

impl eframe::App for DAQApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array() // Make sure we don't paint anything behind the rounded corners
    }
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        // ctx.set_pixels_per_point(self.pixels_per_point);
        ctx.set_style(self.theme.clone());

        // Handle keyboard shortcuts
        let shortcuts = shortcuts::ShortcutHandler::check_shortcuts(ctx);
        for action in shortcuts {
            match action {
                shortcuts::ShortcutAction::ToggleSidebar => {
                    self.is_sidebar_open = !self.is_sidebar_open;
                }
                shortcuts::ShortcutAction::CloseActiveWidget => {
                    self.close_active_widget();
                }
                shortcuts::ShortcutAction::IncreaseScale => {
                    self.pixels_per_point = (self.pixels_per_point + 0.2).min(MAX_UI_SCALE);
                }
                shortcuts::ShortcutAction::DecreaseScale => {
                    self.pixels_per_point = (self.pixels_per_point - 0.2).max(MIN_UI_SCALE);
                }
            }
        }

        ui::sidebar::show(self, ctx);

        workspace::show(self, ctx);

        ctx.request_repaint();
    }
}
