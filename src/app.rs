use crate::{action, can, connection, settings, shortcuts, theme, ui, util, widgets, workspace};
use eframe::egui;

const UI_SCALE_STEP: f32 = 0.2;

pub struct ParserInfo {
    pub dbc_path: std::path::PathBuf,
    pub parser: can_decode::Parser,
}

impl ParserInfo {
    pub fn new(dbc_path: std::path::PathBuf) -> Self {
        let parser =
            can_decode::Parser::from_dbc_file(&dbc_path).expect("Failed to parse DBC file");
        Self { dbc_path, parser }
    }
    pub fn new_maybe(dbc_path: Option<std::path::PathBuf>) -> Option<Self> {
        dbc_path.map(Self::new)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ConnectionStatus {
    Disconnected,
    Connected,
    Error(String),
}

pub struct DAQApp {
    pub connection_status: ConnectionStatus,
    pub is_sidebar_open: bool,
    pub tile_tree: egui_tiles::Tree<widgets::Widget>,
    pub next_can_viewer_num: usize,
    pub next_bootloader_num: usize,
    pub next_scope_num: usize,
    pub next_log_parser_num: usize,
    pub can_receiver: std::sync::mpsc::Receiver<can::can_messages::CanMessage>,
    pub ui_sender: std::sync::mpsc::Sender<ui::ui_messages::UiMessage>,
    pub action_queue: Vec<action::AppAction>,
    pub selected_source: Option<connection::ConnectionSource>,
    pub theme: egui::Style,
    pub theme_selection: theme::ThemeSelection,
    pub pixels_per_point: Option<f32>,
    pub serial_ports: Vec<serialport::SerialPortInfo>,
    pub parser: Option<ParserInfo>,
    pub udp_port: u16,
    pub can_messages: Vec<can::message::ParsedMessage>,
}

impl DAQApp {
    pub fn save_settings(&self) {
        let settings = settings::Settings {
            dbc_path: self.parser.as_ref().map(|p| p.dbc_path.clone()),
            selected_source: self.selected_source.clone(),
            udp_port: self.udp_port,
            theme: self.theme_selection,
            pixels_per_point: self.pixels_per_point,
        };
        settings.save();
    }

    pub fn new(
        can_receiver: std::sync::mpsc::Receiver<can::can_messages::CanMessage>,
        ui_sender: std::sync::mpsc::Sender<ui::ui_messages::UiMessage>,
        settings: settings::Settings,
        _cc: &eframe::CreationContext,
    ) -> Self {
        let theme_selection = settings.theme;
        let theme_style = theme_selection.get_style();

        Self {
            connection_status: ConnectionStatus::Disconnected,
            is_sidebar_open: true,
            tile_tree: egui_tiles::Tree::empty("workspace_tree"),
            next_can_viewer_num: 1,
            next_bootloader_num: 1,
            next_scope_num: 1,
            next_log_parser_num: 1,
            can_receiver,
            ui_sender,
            action_queue: Vec::new(),
            selected_source: settings.selected_source,
            theme: theme_style,
            theme_selection,
            pixels_per_point: settings.pixels_per_point,
            serial_ports: util::get_available_serial_ports(),
            parser: ParserInfo::new_maybe(settings.dbc_path),
            udp_port: settings.udp_port,
            can_messages: Vec::new(),
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

    pub fn connect_can(&mut self) {
        let Some(source) = &self.selected_source else {
            return;
        };

        self.connection_status = ConnectionStatus::Disconnected;

        let _ = self
            .ui_sender
            .send(ui::ui_messages::UiMessage::Connect(source.clone()));
    }

    pub fn handle_action(&mut self, action: action::AppAction, ctx: &egui::Context) {
        match action {
            action::AppAction::SpawnWidget(widget_type) => {
                let widget = match &widget_type {
                    action::WidgetType::ViewerTable => widgets::Widget::ViewerTable(
                        ui::viewer_table::ViewerTable::new(self.next_can_viewer_num),
                    ),
                    action::WidgetType::ViewerList => widgets::Widget::ViewerList(
                        ui::viewer_list::ViewerList::new(self.next_can_viewer_num),
                    ),
                    action::WidgetType::Bootloader => widgets::Widget::Bootloader(
                        ui::bootloader::Bootloader::new(self.next_bootloader_num),
                    ),
                    action::WidgetType::Scope {
                        msg_id,
                        msg_name,
                        signal_name,
                    } => widgets::Widget::Scope(ui::scope::Scope::new(
                        self.next_scope_num,
                        *msg_id,
                        msg_name.clone(),
                        signal_name.clone(),
                    )),
                    action::WidgetType::LogParser => widgets::Widget::LogParser(
                        ui::log_parser::LogParser::new(self.next_log_parser_num),
                    ),
                };
                self.add_widget_to_tree(widget);

                // Increment the appropriate counter
                match widget_type {
                    action::WidgetType::ViewerTable | action::WidgetType::ViewerList => {
                        self.next_can_viewer_num += 1;
                    }
                    action::WidgetType::Bootloader => {
                        self.next_bootloader_num += 1;
                    }
                    action::WidgetType::Scope { .. } => {
                        self.next_scope_num += 1;
                    }
                    action::WidgetType::LogParser => {
                        self.next_log_parser_num += 1;
                    }
                }
            }
            action::AppAction::ToggleSidebar => {
                self.is_sidebar_open = !self.is_sidebar_open;
            }
            action::AppAction::CloseActiveWidget => {
                self.close_active_widget();
            }
            action::AppAction::IncreaseScale => {
                let current_scale = self
                    .pixels_per_point
                    .unwrap_or_else(|| ctx.pixels_per_point());
                self.pixels_per_point = Some(current_scale + UI_SCALE_STEP);
                self.save_settings();
            }
            action::AppAction::DecreaseScale => {
                let current_scale = self
                    .pixels_per_point
                    .unwrap_or_else(|| ctx.pixels_per_point());
                self.pixels_per_point = Some(current_scale - UI_SCALE_STEP);
                self.save_settings();
            }
        }
    }

    pub fn toggle_theme(&mut self) {
        self.theme_selection = self.theme_selection.next();
        self.theme = self.theme_selection.get_style();
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
                can::can_messages::CanMessage::ConnectionFailed(port) => {
                    self.connection_status =
                        ConnectionStatus::Error(format!("Failed to connect to {port}"));
                }
                can::can_messages::CanMessage::ConnectionSuccessful => {
                    self.connection_status = ConnectionStatus::Connected;
                }
                can::can_messages::CanMessage::Disconnection => {
                    self.connection_status = ConnectionStatus::Disconnected;
                }
                can::can_messages::CanMessage::ParsedMessage(parsed) => {
                    self.can_messages.push(parsed.clone());
                }
            }
        }
        if let Some(ppp) = self.pixels_per_point {
            ctx.set_pixels_per_point(ppp);
        }
        ctx.set_style(self.theme.clone());

        // Handle keyboard shortcuts
        self.action_queue
            .extend(shortcuts::ShortcutHandler::check_shortcuts(ctx));

        // Drain the action queue and handle all actions
        for action in std::mem::take(&mut self.action_queue) {
            self.handle_action(action, ctx);
        }

        // Render the most recent state of the UI
        ui::sidebar::show(self, ctx);
        workspace::show(self, ctx);
        ctx.request_repaint();
    }
}
