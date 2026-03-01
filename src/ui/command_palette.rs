use crate::{action, app};
use eframe::egui;

pub fn show(app: &mut app::DAQApp, ctx: &egui::Context) {
    if !app.show_command_palette {
        return;
    }

    egui::Window::new("Command Palette")
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .fixed_size([300.0, 400.0])
        .show(ctx, |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.heading("Command Palette");
                ui.separator();

                let spawn_options = [
                    ("Spawn CAN Table", action::WidgetType::ViewerTable),
                    ("Spawn CAN List", action::WidgetType::ViewerList),
                    (
                        "Spawn Scope",
                        action::WidgetType::Scope {
                            msg_id: 0,
                            msg_name: "".to_string(),
                            signal_name: "".to_string(),
                        },
                    ),
                    ("Spawn Bootloader", action::WidgetType::Bootloader),
                    ("Spawn Log Parser", action::WidgetType::LogParser),
                ];

                for (label, widget_type) in spawn_options {
                    if ui.button(label).clicked() {
                        app.action_queue
                            .push(action::AppAction::SpawnWidget(widget_type));
                        app.show_command_palette = false;
                    }
                }
            });

            // Close on Escape or clicking outside
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                app.show_command_palette = false;
            }

            // Allow closing by clicking on the background (handled by egui naturally with windows)
        });
}
