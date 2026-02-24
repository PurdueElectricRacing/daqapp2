use crate::app::{ConnectionStatus, DAQApp, ThemeSelection};
use crate::can::ConnectionSource;
use crate::widgets::{AppAction, WidgetType};
use eframe::egui;
use serialport::available_ports;

pub fn select_dbc(app: &mut DAQApp) {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("DBC Files", &["dbc"])
        .pick_file()
    {
        app.dbc_path = Some(path.clone());
        app.spawn_can_thread();
        app.save_settings();
    }
}

pub fn show(app: &mut DAQApp, ctx: &egui::Context) {
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
            ui.heading("Customization");

            // Theme toggle button
            let theme_label = match app.theme_selection {
                ThemeSelection::Default => "🎨 Theme: Default",
                ThemeSelection::Nord => "🎨 Theme: Nord",
                ThemeSelection::Catppuccin => "🎨 Theme: Catppuccin",
            };

            if ui.button(theme_label).clicked() {
                app.toggle_theme();
                app.save_settings();
            }

            ui.separator();
            ui.heading("Widgets");

            if ui.button("Spawn CAN Viewer Table").clicked() {
                app.action_queue
                    .push_back(AppAction::SpawnWidget(WidgetType::ViewerTable));
            }

            if ui.button("Spawn CAN Viewer List").clicked() {
                app.action_queue
                    .push_back(AppAction::SpawnWidget(WidgetType::ViewerList));
            }

            if ui.button("Spawn Bootloader").clicked() {
                app.action_queue
                    .push_back(AppAction::SpawnWidget(WidgetType::Bootloader));
            }

            if ui.button("Spawn Log Parser").clicked() {
                app.action_queue
                    .push_back(AppAction::SpawnWidget(WidgetType::LogParser));
            }
            ui.separator();
            ui.heading("Connection Settings");

            ui.horizontal(|ui| {
                ui.label("UDP Port:");
                if ui.add(egui::DragValue::new(&mut app.udp_port).range(1..=65535)).changed() {
                    app.save_settings();
                }
            });

            ui.horizontal(|ui| {
                let selected_text = match &app.selected_source {
                    Some(ConnectionSource::Serial(p)) => format!("Serial: {}", p),
                    Some(ConnectionSource::Udp(p)) => format!("UDP: {}", p),
                    None => "Select Source".to_string(),
                };

                egui::ComboBox::from_label("Source")
                    .selected_text(selected_text)
                    .show_ui(ui, |ui| {
                        ui.label("Serial Ports");
                        let ports: Vec<_> = app.serial_ports.iter().map(|p| p.port_name.clone()).collect();
                        for port_name in ports {
                            let source = ConnectionSource::Serial(port_name.clone());
                            if ui.selectable_value(&mut app.selected_source, Some(source.clone()), &port_name).changed() {
                                app.spawn_can_thread();
                                app.save_settings();
                            }
                        }
                        ui.separator();
                        ui.label("Network");
                        let udp_source = ConnectionSource::Udp(app.udp_port);
                        if ui.selectable_value(&mut app.selected_source, Some(udp_source.clone()), format!("UDP ({})", app.udp_port)).changed() {
                            app.spawn_can_thread();
                            app.save_settings();
                        }
                    });

                if ui.button("🔄").clicked() {
                    app.serial_ports = match available_ports() {
                        Ok(ports) => ports
                            .into_iter()
                            .filter(|p| {
                                let name = p.port_name.to_lowercase();
                                if cfg!(target_os = "windows") {
                                    name.starts_with("com")
                                } else {
                                    name.starts_with("/dev/tty.usbmodem")
                                        || name.starts_with("/dev/ttyacm")
                                }
                            })
                            .collect(),
                        Err(err) => {
                            log::error!("Failed to get ports: {err}");
                            vec![]
                        }
                    };
                }
            });

            ui.horizontal(|ui| {
                if ui.button("🔌 Disconnect").clicked() {
                    app.selected_source = None;
                    app.stop_can_thread();
                    app.save_settings();
                }

                // Connection status indicator
                let (status_icon, status_color) = match &app.connection_status {
                    ConnectionStatus::Disconnected => ("⚪", egui::Color32::GRAY),
                    ConnectionStatus::Connecting => ("🟡", egui::Color32::YELLOW),
                    ConnectionStatus::Connected => ("🟢", egui::Color32::GREEN),
                    ConnectionStatus::Error(_) => ("🔴", egui::Color32::RED),
                };
                ui.label(egui::RichText::new(status_icon).color(status_color));
            });

            if let ConnectionStatus::Error(ref err) = app.connection_status {
                ui.colored_label(egui::Color32::RED, format!(" {err}"));
            }

            ui.horizontal(|ui| {
                if ui.button("📁 Select DBC").clicked() {
                    select_dbc(app);
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
