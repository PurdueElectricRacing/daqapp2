use can_decode;
use eframe::egui;

pub struct LogParser {
    pub title: String,
    dbc_path: Option<std::path::PathBuf>,
    logs_dir: Option<std::path::PathBuf>,
    output_dir: Option<std::path::PathBuf>,
}

impl LogParser {
    pub fn new(instance_num: usize) -> Self {
        Self {
            title: format!("Log Parser #{}", instance_num),
            dbc_path: None,
            logs_dir: None,
            output_dir: None,
        }
    }

    fn select_dbc(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("DBC Files", &["dbc"])
            .pick_file()
        {
            self.dbc_path = Some(path);
        }
    }

    fn select_logs_dir(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            self.logs_dir = Some(path);
        }
    }

    fn select_output_dir(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            self.output_dir = Some(path);
        }
    }

    fn parse_logs(&mut self) {
        let dbc_path = match &self.dbc_path {
            Some(p) => p,
            None => {
                eprintln!("Error: DBC file not selected");
                return;
            }
        };

        let logs_dir = match &self.logs_dir {
            Some(p) => p,
            None => {
                eprintln!("Error: Logs directory not selected");
                return;
            }
        };

        let output_dir = match &self.output_dir {
            Some(p) => p,
            None => {
                eprintln!("Error: Output directory not selected");
                return;
            }
        };

        // TODO: Implement log parsing
        println!("Parsing logs from: {}", logs_dir.display());
        println!("Output to: {}", output_dir.display());
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        ui.heading(format!("🔧 {}", self.title));
        ui.separator();

        // DBC file selection
        ui.horizontal(|ui| {
            if ui.button("📁 Select DBC").clicked() {
                self.select_dbc();
            }
            if let Some(path) = &self.dbc_path {
                ui.label(format!("DBC: {}", path.display()));
            } else {
                ui.label("DBC: None selected");
            }
        });

        ui.separator();

        // Log directory selection
        ui.horizontal(|ui| {
            if ui.button("📁 Select Logs Dir").clicked() {
                self.select_logs_dir();
            }
            if let Some(path) = &self.logs_dir {
                ui.label(format!("Logs: {}", path.display()));
            } else {
                ui.label("Logs: None selected");
            }
        });

        ui.separator();

        // Output directory selection
        ui.horizontal(|ui| {
            if ui.button("📁 Select Output Dir").clicked() {
                self.select_output_dir();
            }
            if let Some(path) = &self.output_dir {
                ui.label(format!("Output: {}", path.display()));
            } else {
                ui.label("Output: None selected");
            }
        });

        ui.separator();

        // Parse button
        if ui.button("▶ Parse Logs").clicked() {
            self.parse_logs();
        }

        egui_tiles::UiResponse::None
    }
}
