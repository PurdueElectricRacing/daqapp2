use eframe::egui;

pub fn show(app: &mut crate::app::DAQApp, ctx: &egui::Context) {
    egui::SidePanel::left("left_sidebar")
        .resizable(true)
        .show_animated(ctx, app.is_sidebar_open, |ui| {
            ui.heading("Side bar");
            ui.separator();
            ui.label("• CAN Viewer\n• Bootloader\n• Live plot");
        });
}