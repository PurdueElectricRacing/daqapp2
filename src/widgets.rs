use crate::{can, ui};
use eframe::egui;

pub enum Widget {
    ViewerTable(ui::viewer_table::ViewerTable),
    ViewerList(ui::viewer_list::ViewerList),
    Bootloader(ui::bootloader::Bootloader),
    Scope(ui::scope::Scope),
    LogParser(ui::log_parser::LogParser),
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
        can_receiver: &std::sync::mpsc::Receiver<can::can_messages::CanMessage>,
        ui_sender: &std::sync::mpsc::Sender<ui::ui_messages::UiMessage>,
        pending_scope_spawns: &mut Vec<(u32, String, String)>,
    ) -> egui_tiles::UiResponse {
        let mut received_new_data = false;

        for msg in can_receiver.try_iter() {
            self.handle_can_message(&msg);
            received_new_data = true;
        }

        // Request repaint only if we received new data
        if received_new_data {
            ui.ctx().request_repaint();
        }

        match self {
            Widget::ViewerTable(w) => w.show(ui, pending_scope_spawns),
            Widget::ViewerList(w) => w.show(ui),
            Widget::Bootloader(w) => w.show(ui),
            Widget::Scope(w) => w.show(ui),
            Widget::LogParser(w) => w.show(ui, ui_sender),
        }
    }

    fn handle_can_message(&mut self, msg: &can::can_messages::CanMessage) {
        match self {
            Widget::ViewerTable(w) => w.handle_can_message(msg),
            Widget::ViewerList(w) => w.handle_can_message(msg),
            Widget::Scope(w) => w.handle_can_message(msg),
            _ => {}
        }
    }
}
