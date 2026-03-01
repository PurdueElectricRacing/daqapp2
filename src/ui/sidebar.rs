use crate::{action, app, connection, ui, util};
use eframe::egui;

pub fn select_dbc(
    app: &mut app::DAQApp,
    ui_sender: &std::sync::mpsc::Sender<ui::ui_messages::UiMessage>,
) {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("DBC Files", &["dbc"])
        .pick_file()
    {
        app.parser = Some(app::ParserInfo::new(path.clone()));
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

            let theme_label = format!("🎨 Theme: {}", app.theme_selection.get_name());

            if ui.button(theme_label).clicked() {
                app.toggle_theme();
                app.save_settings();
            }

            ui.separator();

            if ui.button("Add CAN Viewer Table").clicked() {
                app.action_queue.push(action::AppAction::SpawnWidget(
                    action::WidgetType::ViewerTable,
                ));
            }

            if ui.button("Add CAN Viewer List").clicked() {
                app.action_queue.push(action::AppAction::SpawnWidget(
                    action::WidgetType::ViewerList,
                ));
            }

            if ui.button("Add Bootloader").clicked() {
                app.action_queue.push(action::AppAction::SpawnWidget(
                    action::WidgetType::Bootloader,
                ));
            }

            if ui.button("Add Log Parser").clicked() {
                app.action_queue.push(action::AppAction::SpawnWidget(
                    action::WidgetType::LogParser,
                ));
            }

            ui.separator();
            ui.heading("Connection Settings");

            ui.horizontal(|ui| {
                ui.label("UDP Port:");
                if ui
                    .add(egui::DragValue::new(&mut app.udp_port).range(1..=65535))
                    .changed()
                {
                    app.save_settings();
                }
            });

            ui.horizontal(|ui| {
                let selected_text = match &app.selected_source {
                    Some(connection::ConnectionSource::Serial(p)) => format!("Serial: {}", p),
                    Some(connection::ConnectionSource::Udp(p)) => format!("UDP: {}", p),
                    None => "Select Source".to_string(),
                };

                egui::ComboBox::from_label("Source")
                    .selected_text(selected_text)
                    .show_ui(ui, |ui| {
                        ui.label("Serial Ports");
                        let ports: Vec<_> = app
                            .serial_ports
                            .iter()
                            .map(|p| p.port_name.clone())
                            .collect();
                        for port_name in ports {
                            let source = connection::ConnectionSource::Serial(port_name.clone());
                            if ui
                                .selectable_value(
                                    &mut app.selected_source,
                                    Some(source.clone()),
                                    &port_name,
                                )
                                .changed()
                            {
                                app.connect_can();
                                app.save_settings();
                            }
                        }
                        ui.separator();
                        ui.label("Network");
                        let udp_source = connection::ConnectionSource::Udp(app.udp_port);
                        if ui
                            .selectable_value(
                                &mut app.selected_source,
                                Some(udp_source.clone()),
                                format!("UDP ({})", app.udp_port),
                            )
                            .changed()
                        {
                            app.connect_can();
                            app.save_settings();
                        }
                    });

                if ui.button("🔄").clicked() {
                    app.serial_ports = util::get_avaible_serial_ports();
                }
            });

            ui.horizontal(|ui| {
                // Connection status indicator
                let (status_icon, status_color) = match &app.connection_status {
                    app::ConnectionStatus::Disconnected => ("⚪", egui::Color32::GRAY),
                    app::ConnectionStatus::Connected => ("🟢", egui::Color32::GREEN),
                    app::ConnectionStatus::Error(_) => ("🔴", egui::Color32::RED),
                };
                ui.label(egui::RichText::new(status_icon).color(status_color));
            });

            if let app::ConnectionStatus::Error(ref err) = app.connection_status {
                ui.colored_label(egui::Color32::RED, format!(" {err}"));
            }

            ui.horizontal(|ui| {
                // Clone the sender so we don’t borrow app immutably yet
                let ui_sender = app.ui_sender.clone();

                if ui.button("📁 Select DBC").clicked() {
                    select_dbc(app, &ui_sender); // mutable borrow is fine
                }

                if let Some(path) = app.parser.as_ref().map(|p| &p.dbc_path) {
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
