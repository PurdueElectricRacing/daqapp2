use crate::messages;
use eframe::egui;

pub struct BusLoad {
    pub title: String,
    pub load_1s: f32,
    pub load_5s: f32,
    pub load_10s: f32,
    pub load_30s: f32,
}

impl BusLoad {
    pub fn new(instance: usize) -> Self {
        Self {
            title: format!("Bus Load #{}", instance),
            load_1s: 0.0,
            load_5s: 0.0,
            load_10s: 0.0,
            load_30s: 0.0,
        }
    }

    fn get_color(&self, load: f32) -> egui::Color32 {
        if load < 50.0 {
            egui::Color32::GREEN
        } else if load < 80.0 {
            egui::Color32::YELLOW
        } else {
            egui::Color32::RED
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        egui::Grid::new(format!("bus_load_grid_{}", self.title))
            .striped(true)
            .spacing([20.0, 10.0])
            .show(ui, |ui| {
                // Header
                ui.label(
                    egui::RichText::new("Window")
                        .strong()
                        .color(ui.style().visuals.text_color()),
                );
                ui.label(
                    egui::RichText::new("Bus Load %")
                        .strong()
                        .color(ui.style().visuals.text_color()),
                );
                ui.end_row();

                // 1 second
                ui.label("1 second");
                let color_1s = self.get_color(self.load_1s);
                ui.colored_label(color_1s, format!("{:.2}%", self.load_1s));
                ui.end_row();

                // 5 seconds
                ui.label("5 seconds");
                let color_5s = self.get_color(self.load_5s);
                ui.colored_label(color_5s, format!("{:.2}%", self.load_5s));
                ui.end_row();

                // 10 seconds
                ui.label("10 seconds");
                let color_10s = self.get_color(self.load_10s);
                ui.colored_label(color_10s, format!("{:.2}%", self.load_10s));
                ui.end_row();

                // 30 seconds
                ui.label("30 seconds");
                let color_30s = self.get_color(self.load_30s);
                ui.colored_label(color_30s, format!("{:.2}%", self.load_30s));
                ui.end_row();
            });

        egui_tiles::UiResponse::None
    }

    pub fn handle_can_message(&mut self, msg: &messages::MsgFromCan) {
        if let messages::MsgFromCan::BusLoad {
            load_1s,
            load_5s,
            load_10s,
            load_30s,
        } = msg
        {
            self.load_1s = *load_1s;
            self.load_5s = *load_5s;
            self.load_10s = *load_10s;
            self.load_30s = *load_30s;
        }
    }
}
