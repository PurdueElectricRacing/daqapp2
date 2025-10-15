use eframe::egui;

pub struct CanViewer {
    pub title: String,
}

impl CanViewer {
    pub fn new(instance_num: usize) -> Self {
        Self {
            title: format!("CAN Viewer #{}", instance_num),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        ui.heading(format!("🚗 {}", self.title));
        ui.separator();
        ui.label("CAN message viewer will be implemented here");
        ui.label("• View incoming CAN messages");
        ui.label("• Filter by message ID");
        ui.label("• Decode message content");
        egui_tiles::UiResponse::None
    }
}
