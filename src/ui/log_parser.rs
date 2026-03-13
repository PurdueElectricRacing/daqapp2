use crate::app;
use crate::daq_log_parse;
use eframe::egui;

pub struct LogParser {
    pub title: String,
    pub logs_dir: Option<std::path::PathBuf>,
    pub output_dir: Option<std::path::PathBuf>,

    parse_to_ui_rx: Option<std::sync::mpsc::Receiver<MsgFromParserThread>>,
    parse_text: String,
}

enum MsgFromParserThread {
    FatalExit(String),
    SuccessExit(String),
    Update(String),
}

impl LogParser {
    pub fn new(instance_num: usize) -> Self {
        Self {
            title: format!("Log Parser #{}", instance_num),
            logs_dir: None,
            output_dir: None,
            parse_to_ui_rx: None,
            parse_text: String::new(),
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
                self.parse_text = "Error: Logs directory not selected".to_string();
                return;
            }
        };

        let output_dir = match &self.output_dir {
            Some(p) => p,
            None => {
                log::error!("Error: Output directory not selected");
                self.parse_text = "Error: Output directory not selected".to_string();
                return;
            }
        };

        let parser_info = match parser {
            Some(p) => p,
            None => {
                log::error!("Error: No DBC selected, not parsing");
                self.parse_text = "Error: No DBC selected, not parsing".to_string();
                return;
            }
        };

        // Clone for lifetimes in thread
        let dbc_path = parser_info.dbc_path.clone();
        let logs_dir = logs_dir.clone();
        let output_dir = output_dir.clone();

        let (parse_to_ui_tx, parse_to_ui_rx) = std::sync::mpsc::channel::<MsgFromParserThread>();
        self.parse_to_ui_rx = Some(parse_to_ui_rx);

        std::thread::spawn(move || {
            log::info!("Using DBC: {:?}", dbc_path);
            log::info!("Parsing logs from: {}", logs_dir.display());
            log::info!("Output to: {}", output_dir.display());

            let Ok(parser) = can_decode::Parser::from_dbc_file(&dbc_path) else {
                log::error!("Failed to create CAN parser from DBC file: {:?}", dbc_path);
                parse_to_ui_tx
                    .send(MsgFromParserThread::FatalExit(
                        "Failed to create CAN parser from DBC file".to_string(),
                    ))
                    .unwrap_or_else(|e| {
                        log::error!("Failed to send fatal error message to UI: {}", e)
                    });
                return;
            };

            parse_to_ui_tx
                .send(MsgFromParserThread::Update("Parsing logs...".to_string()))
                .unwrap_or_else(|e| log::error!("Failed to send update message to UI: {}", e));

            let parsed = daq_log_parse::parse::parse_log_files(&logs_dir, &parser);
            let chunked_parsed = daq_log_parse::parse::chunk_parsed(parsed);

            let mut table_builder = daq_log_parse::table::TableBuilder::new();
            table_builder.create_header(&parser);
            table_builder.create_and_write_tables(&output_dir, chunked_parsed);

            log::info!("Parsing completed successfully");
            parse_to_ui_tx
                .send(MsgFromParserThread::SuccessExit(format!(
                    "Parsing completed successfully. Output at: {}",
                    output_dir.display()
                )))
                .unwrap_or_else(|e| log::error!("Failed to send success message to UI: {}", e));
        });
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
        let currently_parsing = self.parse_to_ui_rx.is_some();
        if ui
            .add_enabled(!currently_parsing, egui::Button::new("▶ Parse Logs"))
            .clicked()
        {
            self.parse_logs(parser);
        }

        // Parser thread messages
        if let Some(rx) = &self.parse_to_ui_rx {
            match rx.try_recv() {
                Ok(msg) => match msg {
                    MsgFromParserThread::FatalExit(text) => {
                        self.parse_text = format!("Error: {}", text);
                    }
                    MsgFromParserThread::SuccessExit(text) => {
                        self.parse_text = text;
                    }
                    MsgFromParserThread::Update(text) => {
                        self.parse_text = text;
                    }
                },
                Err(std::sync::mpsc::TryRecvError::Empty) => {} // No message, do nothing
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.parse_to_ui_rx = None;
                }
            }
        }

        ui.separator();
        ui.label(&self.parse_text);

        egui_tiles::UiResponse::None
    }
}
