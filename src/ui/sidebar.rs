use crate::{app, ui, util};
use eframe::egui;

pub fn select_dbc(
    app: &mut app::DAQApp,
    ui_sender: &std::sync::mpsc::Sender<ui::ui_messages::UiMessage>,
) {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("DBC Files", &["dbc"])
        .pick_file()
    {
        app.dbc_path = Some(path.clone());
        ui_sender
            .send(ui::ui_messages::UiMessage::DbcSelected(path))
            .expect("Failed to send DBC selected message");
        app.save_settings();
    }
}

pub fn show(app: &mut app::DAQApp, ctx: &egui::Context) {
    let rounding = if cfg!(target_os = "macos") {
        egui::CornerRadius {
            nw: 12,
            ne: 0,
            sw: 12,
            se: 0,
        }
    } else {
        egui::CornerRadius::ZERO
    };
    egui::SidePanel::left("left_sidebar")
        .frame(
            egui::Frame::new()
                .fill(ctx.style().visuals.window_fill())
                .corner_radius(rounding)
                .inner_margin(10.0),
        )
        .resizable(true)
        .show_animated(ctx, app.is_sidebar_open, |ui| {
            ui.heading("Side bar");
            ui.separator();

            let theme_label = format!("🎨 Theme: {}", app.theme_selection.to_name());

            if ui.button(theme_label).clicked() {
                app.toggle_theme();
                app.save_settings();
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

            if ui.button("Add Log Parser").clicked() {
                app.spawn_log_parser();
            }
            ui.horizontal(|ui| {
                egui::ComboBox::from_label("Serial Port")
                    .selected_text(app.selected_serial.as_deref().unwrap_or("Serial Port"))
                    .show_ui(ui, |ui| {
                        for port in &app.serial_ports {
                            let response = ui.selectable_value(
                                &mut app.selected_serial,
                                Some(port.port_name.clone()),
                                &port.port_name,
                            );
                            if response.changed() {
                                app.ui_sender
                                    .send(ui::ui_messages::UiMessage::SerialSelected(
                                        port.port_name.clone(),
                                    ))
                                    .expect("Failed to send serial selected");
                                app.save_settings();
                            }
                        }
                    });
                if ui.button("🔄").clicked() {
                    app.serial_ports = util::get_avaible_serial_ports();
                }
            });
            if let Some(ref err) = app.connection_error {
                ui.colored_label(egui::Color32::RED, format!(" {err}"));
            }

            ui.horizontal(|ui| {
                // Clone the sender so we don’t borrow app immutably yet
                let ui_sender = app.ui_sender.clone();

                if ui.button("📁 Select DBC").clicked() {
                    select_dbc(app, &ui_sender); // mutable borrow is fine
                }

                // Clone the path for reading only
                let dbc_path = app.dbc_path.clone();

                if let Some(path) = dbc_path {
                    let dbc_name = path
                        .file_name()
                        .map(|n| n.to_string_lossy())
                        .unwrap_or_else(|| path.display().to_string().into());
                    ui.label(format!("{}", dbc_name));
                } else {
                    ui.label("DBC: None selected");
                }
            });
        });
}
