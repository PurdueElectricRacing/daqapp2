use eframe::egui;
use rfd::FileDialog;

pub struct LogParser {
    pub title: String,
    // dbc_path: path,
}

impl LogParser {
    pub fn new(instance_num: usize) -> Self {
        Self {
            title: format!("Log Parser #{}", instance_num),
        }
    }
    
    fn select_dbc() {
        // use rfd to prompt the user to search for .dbc file
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        ui.heading(format!("🔧 {}", self.title));
        ui.separator();
        ui.label("Log parse");
        egui_tiles::UiResponse::None
    }
}