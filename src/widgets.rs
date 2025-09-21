use eframe::egui;
use crate::can_viewer::CanViewer;
use crate::bootloader::Bootloader;
use crate::scope::Scope;

pub enum Widget {
    CanViewer(CanViewer),
    Bootloader(Bootloader),
    Scope(Scope),
}

impl Widget {
    pub fn title(&self) -> &str {
        match self {
            Widget::CanViewer(w) => &w.title,
            Widget::Bootloader(w) => &w.title,
            Widget::Scope(w) => &w.title,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        match self {
            Widget::CanViewer(w) => w.show(ui),
            Widget::Bootloader(w) => w.show(ui),
            Widget::Scope(w) => w.show(ui),
        }
    }
}