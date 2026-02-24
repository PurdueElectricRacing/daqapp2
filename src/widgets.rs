use crate::can::can_messages::CanMessage;
use crate::ui::{
    bootloader::Bootloader, log_parser::LogParser, scope::Scope, viewer_list::ViewerList,
    viewer_table::ViewerTable,
};
use eframe::egui;
use std::{collections::VecDeque, path::PathBuf};

pub enum AppAction {
    SpawnWidget(WidgetType),
    CloseTile(egui_tiles::TileId),
}

pub enum WidgetType {
    ViewerTable,
    ViewerList,
    Bootloader,
    Scope {
        msg_id: u32,
        msg_name: String,
        signal_name: String,
    },
    LogParser,
}

pub enum Widget {
    ViewerTable(ViewerTable),
    ViewerList(ViewerList),
    Bootloader(Bootloader),
    Scope(Scope),
    LogParser(LogParser),
}

impl Widget {
    pub fn title(&self) -> &str {
        match self {
            Widget::ViewerTable(w) => &w.title,
            Widget::ViewerList(w) => &w.title,
            Widget::Bootloader(w) => &w.title,
            Widget::Scope(w) => &w.title,
            Widget::LogParser(w) => &w.title,
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        action_queue: &mut VecDeque<AppAction>,
        dbc_path: Option<&PathBuf>,
    ) -> egui_tiles::UiResponse {
        match self {
            Widget::ViewerTable(w) => w.show(ui, action_queue),
            Widget::ViewerList(w) => w.show(ui, action_queue),
            Widget::Bootloader(w) => w.show(ui, action_queue),
            Widget::Scope(w) => w.show(ui, action_queue),
            Widget::LogParser(w) => w.show(ui, action_queue, dbc_path),
        }
    }

    pub fn handle_can_message(&mut self, msg: &CanMessage) {
        match self {
            Widget::ViewerTable(w) => w.handle_can_message(msg),
            Widget::ViewerList(w) => w.handle_can_message(msg),
            Widget::Scope(w) => w.handle_can_message(msg),
            _ => {}
        }
    }
}
