use crate::{action, app, messages, ui};
use eframe::egui;

pub enum Widget {
    ViewerTable(ui::viewer_table::ViewerTable),
    ViewerList(ui::viewer_list::ViewerList),
    Bootloader(ui::bootloader::Bootloader),
    Scope(ui::scope::Scope),
    LogParser(ui::log_parser::LogParser),
    SendUi(ui::send::SendUi),
}

impl Widget {
    pub fn title(&self) -> &str {
        match self {
            Widget::ViewerTable(w) => &w.title,
            Widget::ViewerList(w) => &w.title,
            Widget::Bootloader(w) => &w.title,
            Widget::Scope(w) => &w.title,
            Widget::LogParser(w) => &w.title,
            Widget::SendUi(w) => &w.title,
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        can_messages: &[messages::ParsedMessage],
        action_queue: &mut Vec<action::AppAction>,
        parser: Option<&app::ParserInfo>,
    ) -> egui_tiles::UiResponse {
        let mut received_new_data = false;

        for msg in can_messages {
            self.handle_can_message(msg);
            received_new_data = true;
        }

        // Request repaint only if we received new data
        if received_new_data {
            ui.ctx().request_repaint();
        }

        match self {
            Widget::ViewerTable(w) => w.show(ui, action_queue),
            Widget::ViewerList(w) => w.show(ui),
            Widget::Bootloader(w) => w.show(ui),
            Widget::Scope(w) => w.show(ui),
            Widget::LogParser(w) => w.show(ui, parser),
            Widget::SendUi(w) => todo!(),
        }
    }

    fn handle_can_message(&mut self, msg: &messages::ParsedMessage) {
        match self {
            Widget::ViewerTable(w) => w.handle_can_message(msg),
            Widget::ViewerList(w) => w.handle_can_message(msg),
            Widget::Scope(w) => w.handle_can_message(msg),
            Widget::SendUi(w) => todo!(),
            _ => {}
        }
    }
}
