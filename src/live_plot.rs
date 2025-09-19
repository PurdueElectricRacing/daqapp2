use eframe::egui;

#[derive(Debug)]
pub struct LivePlot {
    pub title: String,
}

impl LivePlot {
    pub fn new(instance_num: usize) -> Self {
        Self {
            title: format!("Live Plot #{}", instance_num),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        ui.heading(format!("📊 {}", self.title));
        ui.separator();
        ui.label("Live plotting widget will be implemented here");
        ui.label("• Real-time data visualization");
        ui.label("• Multiple signal channels");
        ui.label("• Zoom and pan controls");
        egui_tiles::UiResponse::None
    }
}