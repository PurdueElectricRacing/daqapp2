use crate::ui::{self};
use eframe::egui;
use serialport::available_ports;

pub fn select_dbc(
    app: &mut crate::app::DAQApp,
    ui_sender: &std::sync::mpsc::Sender<ui::ui_messages::UiMessage>,
) {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("DBC Files", &["dbc"])
        .pick_file()
    {
        app.dbc_path = Some(path);
        ui_sender
            .send(ui::ui_messages::UiMessage::DbcSelected(
                app.dbc_path.clone().unwrap(),
            ))
            .expect("Failed to send DBC selected message");
        app.save_settings();
    }
}

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

            egui::ComboBox::from_label("Serial Port")
                .selected_text(app.selected_serial.as_deref().unwrap_or("Serial Port"))
                .show_ui(ui, |ui| {
                    app.serial_ports = match available_ports() {
                        Ok(ports) => ports
                            .into_iter()
                            .filter(|p| {
                                let name = p.port_name.to_lowercase();
                                name.starts_with("/dev/tty.usbmodem")
                                    || name.starts_with("/dev/ttyACM")
                            })
                            .collect(),
                        Err(err) => {
                            eprintln!("Failed to get ports: {err}");
                            vec![]
                        }
                    };

                    for port in &app.serial_ports {
                        ui.selectable_value(
                            &mut app.selected_serial,
                            Some(port.port_name.clone()),
                            &port.port_name,
                        );
                        app.ui_sender
                            .send(ui::ui_messages::UiMessage::SerialSelected(
                                port.port_name.clone(),
                            ))
                            .expect("Failed to send serial selected");
                        app.save_settings();
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
