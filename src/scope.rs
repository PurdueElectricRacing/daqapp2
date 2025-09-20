use std::collections::VecDeque;
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};

#[derive(Debug)]
pub struct Scope {
    pub title: String,
    window_size: usize,
    window: VecDeque<f64>,
    signal_name: String,
    is_paused: bool,
}

impl Scope {
    pub fn new(instance_num: usize) -> Self {
        Self {
            title: format!("Scope #{}", instance_num),
            window_size: 1000,
            window: VecDeque::new(),
            signal_name: String::from("Signal"),
            is_paused: false,
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

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        ui.heading(format!("📊 {}: {}", self.title, self.signal_name));
        
        // Horizontal container
        ui.horizontal(|ui| {
            // Pause/Resume button
            let pause_text = if self.is_paused { "▶ Resume" } else { "⏸ Pause" };
            if ui.button(pause_text).clicked() {
                self.is_paused = !self.is_paused;
            }
            
            ui.separator();
            
            // Window size slider
            ui.label("Window Size:");
            ui.add(egui::Slider::new(&mut self.window_size, 10..=10000).suffix(" samples"));
            
            ui.separator();
            
            // Export button
            if ui.button("📄 Export CSV").clicked() {
                // TODO: Implement CSV export functionality
            }
            
            ui.separator();
            
            // Clear button
            if ui.button("🗑 Clear").clicked() {
                self.window.clear();
            }
        });

        ui.separator();

        Plot::new(&self.title)
            .view_aspect(2.0)
            .auto_bounds(egui::Vec2b::TRUE)
            .show(ui, |plot_ui| {
                if !self.window.is_empty() {
                    let points: PlotPoints = self.window
                        .iter()
                        .enumerate()
                        .map(|(i, &value)| [i as f64, value])
                        .collect();
                    
                    // Create line with nice styling
                    let line = Line::new(&self.signal_name, points)
                        .color(egui::Color32::from_rgb(100, 200, 100))
                        .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 200, 100)));
                    
                    plot_ui.line(line);
                }
            });

        egui_tiles::UiResponse::None
    }
}