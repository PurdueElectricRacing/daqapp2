use crate::app;
use crate::daq_log_parse;
use eframe::egui;

pub struct LogParser {
    pub title: String,
    pub logs_dir: Option<std::path::PathBuf>,
    pub output_dir: Option<std::path::PathBuf>,
}

impl LogParser {
    pub fn new(instance_num: usize) -> Self {
        Self {
            title: format!("Log Parser #{}", instance_num),
            logs_dir: None,
            output_dir: None,
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

    fn parse_logs(&mut self, parser: Option<&app::ParserInfo>) {
        let logs_dir = match &self.logs_dir {
            Some(p) => p,
            None => {
                // TODO: make persistent log directories
                log::error!("Error: Logs directory not selected");
                return;
            }
        };

        let output_dir = match &self.output_dir {
            Some(p) => p,
            None => {
                log::error!("Error: Output directory not selected");
                return;
            }
        };

        match parser {
            Some(p) => {
                // TODO: make proper UI indication that parse has been completed/successful

                let dbc_path = p.dbc_path.clone();
                let logs_dir = logs_dir.clone();
                let output_dir = output_dir.clone();

                std::thread::spawn(move || {
                    log::info!("Using DBC: {:?}", dbc_path);
                    log::info!("Parsing logs from: {}", logs_dir.display());
                    log::info!("Output to: {}", output_dir.display());

                    let Ok(parser) = can_decode::Parser::from_dbc_file(&dbc_path) else {
                        log::error!("Failed to create CAN parser from DBC file: {:?}", dbc_path);
                        return;
                    };

                    let parsed = daq_log_parse::parse::parse_log_files(&logs_dir, &parser);
                    let chunked_parsed = daq_log_parse::parse::chunk_parsed(parsed);

                    let mut table_builder = daq_log_parse::table::TableBuilder::new();
                    table_builder.create_header(&parser);
                    table_builder.create_and_write_tables(&output_dir, chunked_parsed);

                    log::info!("Parsing completed successfully");
                });
            }
            // TODO: make proper UI indication that parse has failed / not occured
            None => log::warn!("No DBC selected, not parsing"),
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        parser: Option<&app::ParserInfo>,
    ) -> egui_tiles::UiResponse {
        ui.heading(format!("🔧 {}", self.title));
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
            self.parse_logs(parser);
        }

        egui_tiles::UiResponse::None
    }
}
