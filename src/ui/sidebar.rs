use eframe::egui;

pub fn show(app: &mut crate::app::DAQApp, ctx: &egui::Context) {
    egui::SidePanel::left("left_sidebar")
        .resizable(true)
        .show_animated(ctx, app.is_sidebar_open, |ui| {
            ui.heading("Side bar");
            ui.separator();

            // Theme toggle button
            let theme_label = match app.theme_selection {
                crate::app::ThemeSelection::Default => "🎨 Theme: Default",
                crate::app::ThemeSelection::Nord => "🎨 Theme: Nord",
                crate::app::ThemeSelection::Catppuccin => "🎨 Theme: Catppuccin",
            };
            
            if ui.button(theme_label).clicked() {
                app.toggle_theme();
            }

            ui.separator();

            if ui.button("Add CAN Viewer Table").clicked() {
                app.spawn_viewer_table();
            }

            if ui.button("Add CAN Viewer List").clicked() {
                app.spawn_viewer_list();
            }

            if ui.button("Add Bootloader").clicked() {
                app.spawn_bootloader();
            }

            if ui.button("Add Scope").clicked() {
                app.spawn_scope();
            }

            if ui.button("Add Log Parser").clicked() {
                app.spawn_log_parser();
            }
        });
}
