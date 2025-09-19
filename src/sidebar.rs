use eframe::egui;

pub fn show(app: &mut crate::app::DAQApp, ctx: &egui::Context) {
    egui::SidePanel::left("left_sidebar")
        .resizable(true)
        .show_animated(ctx, app.is_sidebar_open, |ui| {
            ui.heading("Side bar");
            ui.separator();
            
            // Widget spawn controls
            ui.heading("Spawn Widgets");
            
            if ui.button("Add CAN Viewer").clicked() {
                app.spawn_can_viewer();
            }
            
            if ui.button("Add Bootloader").clicked() {
                app.spawn_bootloader();
            }
            
            if ui.button("Add Live Plot").clicked() {
                app.spawn_live_plot();
            }
        });
}