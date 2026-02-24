use crate::can::{self, can_messages::CanMessage, ConnectionSource};
use crate::widgets::{AppAction, Widget, WidgetType};
use crate::{config, shortcuts, ui, workspace};
use eframe::egui;
use serde::{Deserialize, Serialize};
use serialport::available_ports;
use std::{
    collections::VecDeque,
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender},
        Arc,
    },
    thread::JoinHandle,
};

pub(crate) const SETTINGS_PATH: &str = "settings.json";
const NORD_THEME_PATH: &str = "themes/nord.toml";
const CATPPUCCIN_THEME_PATH: &str = "themes/catppuccin.toml";
#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub enum ThemeSelection {
    Default,
    Nord,
    Catppuccin,
}

pub struct DAQApp {
    pub is_sidebar_open: bool,
    pub tile_tree: egui_tiles::Tree<Widget>,
    pub next_instance_num: usize,
    pub can_receiver: Receiver<CanMessage>,
    pub action_queue: VecDeque<AppAction>,
    pub theme: egui::Style,
    pub theme_selection: ThemeSelection,
    pub pixels_per_point: f32,
    pub selected_source: Option<ConnectionSource>,
    pub udp_port: u16,
    pub serial_ports: Vec<serialport::SerialPortInfo>,
    pub dbc_path: Option<PathBuf>,
    pub connection_error: Option<String>,
    pub can_messages: Vec<CanMessage>,
    pub can_thread: Option<JoinHandle<()>>,
    pub stop_signal: Arc<AtomicBool>,
    pub can_sender: Sender<CanMessage>,
}

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub dbc_path: Option<PathBuf>,
    pub selected_source: Option<ConnectionSource>,
    pub udp_port: u16,
    pub theme: ThemeSelection,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeSelection::Default,
            dbc_path: None,
            selected_source: None,
            udp_port: 5000,
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
        fs::write(path, json).unwrap_or_else(|e| log::error!("Failed to write {}: {}", path, e));
    }
}

const MIN_UI_SCALE: f32 = 0.4;
const MAX_UI_SCALE: f32 = 5.0;

impl DAQApp {
    pub fn save_settings(&self) {
        let settings = Settings {
            dbc_path: self.dbc_path.clone(),
            selected_source: self.selected_source.clone(),
            udp_port: self.udp_port,
            theme: self.theme_selection,
        };
        settings.save(SETTINGS_PATH);
    }

    pub fn new(
        can_receiver: Receiver<CanMessage>,
        can_sender: Sender<CanMessage>,
        cc: &eframe::CreationContext,
    ) -> Self {
        // Calculate a default ui scale based off the native_pixels_per_point
        let native_ppp = cc.egui_ctx.native_pixels_per_point().unwrap_or(1.0);
        let default_scale = (native_ppp * 2.4).clamp(MIN_UI_SCALE, MAX_UI_SCALE);
        let settings = Settings::load(SETTINGS_PATH);
        let theme_selection = settings.theme;
        let theme = match theme_selection {
            ThemeSelection::Default => egui::Style::default(),
            ThemeSelection::Nord => config::ThemeColors::load_from_file(NORD_THEME_PATH)
                .map(|t| t.to_egui_style())
                .unwrap_or_default(),
            ThemeSelection::Catppuccin => {
                config::ThemeColors::load_from_file(CATPPUCCIN_THEME_PATH)
                    .map(|t| t.to_egui_style())
                    .unwrap_or_default()
            }
        };

        let selected_source = settings.selected_source.clone();
        let udp_port = settings.udp_port;
        let dbc_path = settings.dbc_path.clone();
        let mut app = Self {
            is_sidebar_open: true,
            tile_tree: egui_tiles::Tree::empty("workspace_tree"),
            next_instance_num: 1,
            can_receiver,
            action_queue: VecDeque::new(),
            theme,
            theme_selection,
            pixels_per_point: default_scale,
            serial_ports: available_ports()
                .unwrap_or_default()
                .into_iter()
                .filter(|p| {
                    let name = p.port_name.to_lowercase();
                    if cfg!(target_os = "windows") {
                        name.starts_with("com")
                    } else {
                        name.starts_with("/dev/tty.usbmodem") || name.starts_with("/dev/ttyacm")
                    }
                })
                .collect(),
            selected_source,
            udp_port,
            dbc_path,
            connection_error: None,
            can_messages: Vec::new(),
            can_thread: None,
            stop_signal: Arc::new(AtomicBool::new(false)),
            can_sender,
        };

        // If we had a saved source, try to connect immediately
        if app.selected_source.is_some() {
            app.spawn_can_thread();
        }

        app
    }

    pub fn stop_can_thread(&mut self) {
        self.stop_signal
            .store(true, Ordering::Relaxed);
        if let Some(handle) = self.can_thread.take() {
            let _ = handle.join();
        }
        self.stop_signal
            .store(false, Ordering::Relaxed);
    }

    pub fn spawn_can_thread(&mut self) {
        self.stop_can_thread();

        let Some(source) = &self.selected_source else {
            return;
        };

        let driver = match source.create_driver() {
            Ok(d) => d,
            Err(e) => {
                self.connection_error = Some(format!("Error: {e}"));
                return;
            }
        };

        self.can_thread = Some(can::thread::spawn_worker(
            self.can_sender.clone(),
            driver,
            self.dbc_path.clone(),
            self.stop_signal.clone(),
        ));
    }

    fn add_widget_to_tree(&mut self, widget: Widget) {
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

    pub fn handle_action(&mut self, action: AppAction) {
        match action {
            AppAction::SpawnWidget(ty) => self.spawn_widget(ty),
            AppAction::CloseTile(tile_id) => {
                self.tile_tree.tiles.remove(tile_id);
            }
        }
    }

    pub fn spawn_widget(&mut self, ty: WidgetType) {
        let widget = match ty {
            WidgetType::ViewerTable => {
                let w = Widget::ViewerTable(ui::viewer_table::ViewerTable::new(
                    self.next_instance_num,
                ));
                self.next_instance_num += 1;
                w
            }
            WidgetType::ViewerList => {
                // Fix: ViewerList should use its own type name in labels, but for now we follow the existing pattern
                let w = Widget::ViewerList(ui::viewer_list::ViewerList::new(
                    self.next_instance_num,
                ));
                self.next_instance_num += 1;
                w
            }
            WidgetType::Bootloader => {
                let w = Widget::Bootloader(ui::bootloader::Bootloader::new(
                    self.next_instance_num,
                ));
                self.next_instance_num += 1;
                w
            }
            WidgetType::Scope {
                msg_id,
                msg_name,
                signal_name,
            } => {
                let w = Widget::Scope(ui::scope::Scope::new(
                    self.next_instance_num,
                    msg_id,
                    msg_name,
                    signal_name,
                ));
                self.next_instance_num += 1;
                w
            }
            WidgetType::LogParser => {
                let w = Widget::LogParser(ui::log_parser::LogParser::new(
                    self.next_instance_num,
                ));
                self.next_instance_num += 1;
                w
            }
        };
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
            ThemeSelection::Nord => config::ThemeColors::load_from_file(NORD_THEME_PATH)
                .map(|t| t.to_egui_style())
                .unwrap_or_default(),
            ThemeSelection::Catppuccin => {
                config::ThemeColors::load_from_file(CATPPUCCIN_THEME_PATH)
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
        self.can_messages.clear();
        while let Ok(msg) = self.can_receiver.try_recv() {
            match &msg {
                CanMessage::ConnectionFailed(port) => {
                    self.connection_error = Some(format!("Failed to connect to {port}"));
                }
                CanMessage::ConnectionSuccessful => {
                    self.connection_error = None;
                }
                _ => {
                    self.can_messages.push(msg);
                }
            }
        }

        // 1. Deliver data to all widgets (even non-visible ones)
        if !self.can_messages.is_empty() {
            for tile in self.tile_tree.tiles.tiles_mut() {
                if let egui_tiles::Tile::Pane(widget) = tile {
                    for msg in &self.can_messages {
                        widget.handle_can_message(msg);
                    }
                }
            }
            ctx.request_repaint(); // Ensure we redraw if we got data
        }

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
