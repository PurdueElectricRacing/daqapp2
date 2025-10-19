use crate::{can, ui};
use eframe::egui;

pub enum Widget {
    ViewerTable(ui::viewer_table::ViewerTable),
    Bootloader(ui::bootloader::Bootloader),
    Scope(ui::scope::Scope),
    LogParser(ui::log_parser::LogParser),
}

impl Widget {
    pub fn title(&self) -> &str {
        match self {
            Widget::ViewerTable(w) => &w.title,
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
    ) -> egui_tiles::UiResponse {
        for msg in can_receiver.try_iter() {
            self.handle_can_message(&msg);
        }

        match self {
            Widget::ViewerTable(w) => w.show(ui),
            Widget::Bootloader(w) => w.show(ui),
            Widget::Scope(w) => w.show(ui),
            Widget::LogParser(w) => w.show(ui, ui_sender),
        }
    }

    fn handle_can_message(&mut self, msg: &can::can_messages::CanMessage) {
        match self {
            Widget::ViewerTable(w) => w.handle_can_message(msg),
            _ => {}
        }
    }
}
