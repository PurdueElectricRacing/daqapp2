use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use std::collections::VecDeque;

// TODO add trigger
// TODO fft view

pub struct Scope {
    pub title: String,
    msg_id: u32,
    msg_name: String,
    signal_name: String,
    window: VecDeque<(f64, f64)>, // (time, value)
    window_duration_seconds: f64,
    reference_time: Option<chrono::DateTime<chrono::Local>>,
    is_paused: bool,
}

impl Scope {
    pub fn new(instance_num: usize, msg_id: u32, msg_name: String, signal_name: String) -> Self {
        let title = format!("Scope #{}", instance_num);
        Self {
            title,
            msg_id,
            msg_name,
            signal_name,
            window: VecDeque::new(),
            window_duration_seconds: 10.0, // Default 10 seconds
            reference_time: None,
            is_paused: false,
        }
    }

    pub fn add_point(&mut self, timestamp: chrono::DateTime<chrono::Local>, value: f64) {
        if self.is_paused {
            return;
        }

        // Initialize reference time on first sample
        if self.reference_time.is_none() {
            self.reference_time = Some(timestamp);
        }

        let reference = self.reference_time.unwrap();

        // Calculate relative time in seconds
        let relative_time = (timestamp - reference).num_milliseconds() as f64 / 1000.0;

        self.window.push_back((relative_time, value));

        // Remove old data outside time window
        let cutoff_time = relative_time - self.window_duration_seconds;
        while let Some((oldest_time, _)) = self.window.front() {
            if *oldest_time < cutoff_time {
                self.window.pop_front();
            } else {
                break;
            }
        }
    }

    fn export_csv(&self) {
        // Create CSV content from the window data
        let mut csv_content = String::from("Time_Seconds,Value\n");
        for (relative_time, value) in &self.window {
            csv_content.push_str(&format!("{},{}\n", relative_time, value));
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
        ui.heading(format!(
            "📊 {}: {} - {}",
            self.title, self.msg_name, self.signal_name
        ));

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

            // Window duration slider
            ui.label("Window Duration:");
            ui.add(
                egui::Slider::new(&mut self.window_duration_seconds, 1.0..=120.0)
                    .suffix(" seconds"),
            );

            ui.separator();

            // Export button
            if ui.button("📄 Export CSV").clicked() {
                self.export_csv();
            }

            ui.separator();

            // Clear button
            if ui.button("🗑 Clear").clicked() {
                self.window.clear();
                self.reference_time = None;
            }

            ui.separator();
        });

        ui.separator();

        Plot::new(&self.title)
            .view_aspect(2.0)
            .auto_bounds(egui::Vec2b::TRUE)
            .x_axis_label("Time (seconds)")
            .y_axis_label(&self.signal_name)
            .show(ui, |plot_ui| {
                if self.window.is_empty() {
                    return;
                }

                let points: PlotPoints = self
                    .window
                    .iter()
                    .map(|(time, value)| [*time, *value])
                    .collect();

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

        self.add_point(parsed.timestamp, signal.value);
    }
}
