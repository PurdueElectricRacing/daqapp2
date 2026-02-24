use crate::widgets::AppAction;
use eframe::egui;
use std::collections::VecDeque;

pub struct Bootloader {
    pub title: String,
}

impl Bootloader {
    pub fn new(instance_num: usize) -> Self {
        Self {
            title: format!("Bootloader #{}", instance_num),
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        _action_queue: &mut VecDeque<AppAction>,
    ) -> egui_tiles::UiResponse {
        ui.heading(format!("🔧 {}", self.title));
        ui.separator();
        ui.label("Bootloader interface will be implemented here");
        ui.label("• Flash firmware updates");
        ui.label("• Verify checksums");
        ui.label("• Monitor progress");
        egui_tiles::UiResponse::None
    }
}
