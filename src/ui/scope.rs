use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use std::collections::VecDeque;

// TODO time based instead of sample based?
// TODO add trigger
// TODO fft view

pub struct Scope {
    pub title: String,
    msg_id: u32,
    signal_name: String,
    window: VecDeque<f64>,
    window_size: usize,
    is_paused: bool,
    show_line: bool,
}

impl Scope {
    pub fn new(instance_num: usize, msg_id: u32, signal_name: String) -> Self {
        let title = format!("Scope #{}: {}", instance_num, &signal_name);
        Self {
            title,
            msg_id,
            signal_name,
            window: VecDeque::new(),
            window_size: 1000,
            is_paused: false,
            show_line: false,
        }
    }

    pub fn add_point(&mut self, value: f64) {
        if self.is_paused {
            return;
        }

        self.window.push_back(value);

        if self.window.len() > self.window_size {
            self.window.pop_front();
        }
    }

    fn export_csv(&self) {
        // Create CSV content from the window data
        let mut csv_content = String::from("Sample,Value\n");

        for (index, &value) in self.window.iter().enumerate() {
            csv_content.push_str(&format!("{},{}\n", index, value));
        }

        // Open file dialog to save CSV
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name(format!("{}_data.csv", self.title.replace(" ", "_")))
            .add_filter("CSV Files", &["csv"])
            .save_file()
        {
            if let Err(e) = std::fs::write(&path, csv_content) {
                eprintln!("Failed to save CSV file: {}", e);
            } else {
                println!("CSV exported to: {}", path.display());
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        ui.heading(format!("📊 {}: {}", self.title, self.signal_name));

        // Horizontal container
        ui.horizontal(|ui| {
            // Pause/Resume button
            let pause_text = if self.is_paused {
                "▶ Resume"
            } else {
                "⏸ Pause"
            };
            if ui.button(pause_text).clicked() {
                self.is_paused = !self.is_paused;
            }

            ui.separator();

            // Window size slider
            ui.label("Window Size:");
            ui.add(egui::Slider::new(&mut self.window_size, 10..=1000).suffix(" samples"));

            ui.separator();

            // Export button
            if ui.button("📄 Export CSV").clicked() {
                self.export_csv();
            }

            ui.separator();

            // Clear button
            if ui.button("🗑 Clear").clicked() {
                self.window.clear();
            }

            ui.separator();

            // Show line checkbox
            ui.checkbox(&mut self.show_line, "Show Line");
        });

        ui.separator();

        Plot::new(&self.title)
            .view_aspect(2.0)
            .auto_bounds(egui::Vec2b::TRUE)
            .show(ui, |plot_ui| {
                if self.window.is_empty() {
                    return;
                }

                let points: PlotPoints = self
                    .window
                    .iter()
                    .enumerate()
                    .map(|(i, &value)| [i as f64, value])
                    .collect();

                if !self.show_line {
                    return;
                }

                let line = Line::new(&self.signal_name, points)
                    .color(egui::Color32::from_rgb(100, 200, 100))
                    .stroke(egui::Stroke::new(
                        2.0,
                        egui::Color32::from_rgb(100, 200, 100),
                    ));

                plot_ui.line(line);
            });

        egui_tiles::UiResponse::None
    }

    pub fn handle_can_message(&mut self, msg: &crate::can::can_messages::CanMessage) {
        let crate::can::can_messages::CanMessage::ParsedMessage(parsed) = msg;
        
        if parsed.decoded.msg_id != self.msg_id {
            return;
        }
        
        let Some(signal) = parsed.decoded.signals.get(&self.signal_name) else {
            return;
        };
        
        self.add_point(signal.value);
    }
}
