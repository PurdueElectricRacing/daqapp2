use eframe::egui;

pub struct DAQApp {
    pub is_sidebar_open: bool
}

impl Default for DAQApp {
    fn default() -> Self {
        Self {
            is_sidebar_open: true
        }
    }
}

impl eframe::App for DAQApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        // a tiny toolbar button to toggle the sidebar
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            if ui.button(if self.is_sidebar_open { "Hide side bar" } else { "Show sidebar" }).clicked() {
                self.is_sidebar_open = !self.is_sidebar_open;
            }
        });

        // sidebar
        crate::sidebar::show(self, ctx);

        // center
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Hello, egui!");
        });
    }
}