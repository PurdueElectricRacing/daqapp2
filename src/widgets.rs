use crate::bootloader::Bootloader;
use crate::can_viewer::CanViewer;
use crate::log_parser::LogParser;
use crate::scope::Scope;
use eframe::egui;

pub enum Widget {
    CanViewer(CanViewer),
    Bootloader(Bootloader),
    Scope(Scope),
    LogParser(LogParser),
}

impl Widget {
    pub fn title(&self) -> &str {
        match self {
            Widget::CanViewer(w) => &w.title,
            Widget::Bootloader(w) => &w.title,
            Widget::Scope(w) => &w.title,
            Widget::LogParser(w) => &w.title,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        match self {
            Widget::CanViewer(w) => w.show(ui),
            Widget::Bootloader(w) => w.show(ui),
            Widget::Scope(w) => w.show(ui),
            Widget::LogParser(w) => w.show(ui),
        }
    }
}
