use crate::{messages, theme};
use eframe::egui::{self, Color32, Frame, RichText, Stroke, Vec2};

pub struct ChargeController {
    pub title: String,

    pub max_charge_voltage: u16,
    pub max_charge_current: u16,
    pub charge_enable: bool,

    pub charge_voltage: u16,
    pub charge_current: u16,

    pub hardware_fault: bool,
    pub temperature_fail: bool,
    pub input_voltage_fault: bool,
    pub start_fail: bool,
    pub communication_fault: bool,

    pub last_update: std::time::Instant,
    pub is_data_stale: bool,
    pub timeout_seconds: u64,

    pub command_msg_id: u32,
    pub status_msg_id: u32,

    pub command_msg_period: usize, // default 1 second, from elcon datasheet
}

impl ChargeController {
    pub fn new() -> Self {
        Self {
            title: "Charge Controller".to_string(),
            max_charge_voltage: 0,
            max_charge_current: 0,
            charge_enable: false,
            charge_voltage: 0,
            charge_current: 0,
            hardware_fault: false,
            temperature_fail: false,
            input_voltage_fault: false,
            start_fail: false,
            communication_fault: false,
            last_update: std::time::Instant::now() - std::time::Duration::from_secs(10), // start as stale
            is_data_stale: true,
            timeout_seconds: 2,
            command_msg_id: 0x1806E5F4,
            status_msg_id: 0x98FF50E5, // for some reason definition in dbc is eid? override has correct number but idk
            command_msg_period: 1000,
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        ui_to_can_tx: std::sync::mpsc::Sender<messages::MsgFromUi>,
    ) -> egui_tiles::UiResponse {
        // Pull theme colors from global ctx storage
        let theme = theme::get_theme(ui.ctx());

        self.is_data_stale = self.last_update.elapsed() > std::time::Duration::from_secs(self.timeout_seconds);
        let stale = self.is_data_stale;
        let elapsed = self.last_update.elapsed().as_secs_f32();

        ui.vertical(|ui| {
            // ── Title ──────────────────────────────────────────────────────
            ui.add_space(4.0);
            ui.heading(&self.title);
            ui.add_space(4.0);

            // ── Stale / Live Banner ────────────────────────────────────────
            {
                let (banner_color, dot_color, banner_text) = if stale {
                    (
                        // muted orange tint background
                        Color32::from_rgba_unmultiplied(
                            theme.warning_color().r(),
                            theme.warning_color().g(),
                            theme.warning_color().b(),
                            30,
                        ),
                        theme.warning_color(),
                        format!(
                            "No broadcast received — last message {:.1} s ago  ·  fault states and readings invalid",
                            elapsed
                        ),
                    )
                } else {
                    (
                        Color32::from_rgba_unmultiplied(
                            theme.success_color().r(),
                            theme.success_color().g(),
                            theme.success_color().b(),
                            30,
                        ),
                        theme.success_color(),
                        format!("Broadcast received — last message {:.1} s ago", elapsed),
                    )
                };

                Frame::NONE
                    .fill(banner_color)
                    .stroke(Stroke::new(1.0, dot_color.linear_multiply(0.5)))
                    .inner_margin(egui::Margin::symmetric(10, 6))
                    .corner_radius(4.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Dot indicator
                            let (rect, _) = ui.allocate_exact_size(
                                Vec2::splat(8.0),
                                egui::Sense::hover(),
                            );
                            ui.painter().circle_filled(rect.center(), 4.0, dot_color);
                            ui.add_space(4.0);
                            ui.colored_label(dot_color, &banner_text);
                        });
                    });
            }

            ui.add_space(8.0);

            // split ui into two columns 
            ui.columns(2, |cols| {
            let (left_slice, right_slice) = cols.split_at_mut(1);
            let left  = &mut left_slice[0];
            let right = &mut right_slice[0];

                // LEFT: fault states & charge command 
                left.vertical(|ui| {
                    // fault panel
                    Frame::NONE
                        .fill(theme.panel_color())
                        .stroke(Stroke::new(1.0, theme.accent_color()))
                        .inner_margin(egui::Margin::same(10))
                        .corner_radius(4.0)
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("FAULT STATES  (0x18FF50E5 · Byte 5)")
                                    .size(10.0)
                                    .color(theme.text_color().linear_multiply(0.5)),
                            );
                            ui.add_space(6.0);

                            Self::fault_pill(ui, &theme, "Hardware",      self.hardware_fault,       stale);
                            Self::fault_pill(ui, &theme, "Temperature",   self.temperature_fail,     stale);
                            Self::fault_pill(ui, &theme, "Input Voltage", self.input_voltage_fault,  stale);
                            Self::fault_pill(ui, &theme, "Startup",       self.start_fail,           stale);
                            Self::fault_pill(ui, &theme, "Communication", self.communication_fault,  stale);
                        });

                    ui.add_space(10.0);

                    // charge command inputs
                    ui.label(
                        RichText::new(format!("CHARGE COMMAND  (0x{:X})", self.command_msg_id))
                            .size(10.0)
                            .color(theme.text_color().linear_multiply(0.5)),
                    );
                    ui.add_space(4.0);

                    Self::input_row(ui, &theme, "Max Voltage", &mut self.max_charge_voltage, "V", 0..=1000);
                    ui.add_space(4.0);
                    Self::input_row(ui, &theme, "Max Current", &mut self.max_charge_current, "A", 0..=500);
                    ui.add_space(8.0);

                    // charge enable toggle button (full width)
                    let (toggle_text, toggle_color) = if self.charge_enable {
                        ("● Charge Output Enabled", theme.success_color())
                    } else {
                        ("○ Charge Output Disabled", theme.text_color().linear_multiply(0.4))
                    };

                    let toggle_bg = if self.charge_enable {
                        Color32::from_rgba_unmultiplied(
                            theme.success_color().r(),
                            theme.success_color().g(),
                            theme.success_color().b(),
                            30,
                        )
                    } else {
                        theme.panel_color()
                    };

                    let toggle_btn = Frame::NONE
                        .fill(toggle_bg)
                        .stroke(Stroke::new(1.0, toggle_color))
                        .inner_margin(egui::Margin::symmetric(10, 7))
                        .corner_radius(4.0)
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.centered_and_justified(|ui| {
                                ui.colored_label(toggle_color, toggle_text);
                            });
                        });

                    // charge command send enable/disable logic
                    if toggle_btn.response.interact(egui::Sense::click()).clicked() {
                        self.charge_enable = !self.charge_enable;
                        if (self.charge_enable) {
                            self.send_charge_command(&ui_to_can_tx);
                        }
                        else {
                            self.stop_charge_command(&ui_to_can_tx);
                        }
                    }

                    ui.add_space(6.0);
                });

                // RIGHT: Reading cards 
                right.vertical(|ui| {
                    ui.label(
                        RichText::new(format!("CHARGER OUTPUT  (0x{:X})", self.status_msg_id))
                            .size(10.0)
                            .color(theme.text_color().linear_multiply(0.5)),
                    );
                    ui.add_space(4.0);

                    ui.columns(2, |rcols| {
                        Self::reading_card(
                            &mut rcols[0],
                            &theme,
                            "OUTPUT V",
                            self.charge_voltage as f32 / 10.0,
                            "V",
                            stale,
                        );
                        Self::reading_card(
                            &mut rcols[1],
                            &theme,
                            "OUTPUT A",
                            self.charge_current as f32 / 10.0,
                            "A",
                            stale,
                        );
                    });

                    // Graph placeholder — egui_plot goes here later
                    ui.add_space(10.0);
                    Frame::NONE
                        .fill(theme.panel_color())
                        .stroke(Stroke::new(1.0, theme.accent_color()))
                        .inner_margin(egui::Margin::same(10))
                        .corner_radius(4.0)
                        .show(ui, |ui| {
                            ui.set_min_height(120.0);
                            ui.set_min_width(ui.available_width());
                            ui.centered_and_justified(|ui| {
                                ui.colored_label(
                                    theme.text_color().linear_multiply(0.3),
                                    "graph placeholder idk",
                                );
                            });
                        });
                });
            });
        });

        egui_tiles::UiResponse::None
    }

    // helper functions

    /// Renders a fault row: label on the left, colored pill on the right.
    fn fault_pill(
        ui: &mut egui::Ui,
        theme: &theme::ThemeColors,
        label: &str,
        fault: bool,
        stale: bool,
    ) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(label).size(12.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (pill_text, pill_color) = if stale {
                    ("STALE", theme.text_color().linear_multiply(0.35))
                } else if fault {
                    ("FAULT", theme.error_color())
                } else {
                    ("OK", theme.success_color())
                };

                let pill_bg = Color32::from_rgba_unmultiplied(
                    pill_color.r(), pill_color.g(), pill_color.b(), 40,
                );

                Frame::NONE
                    .fill(pill_bg)
                    .stroke(Stroke::new(1.0, pill_color.linear_multiply(0.6)))
                    .inner_margin(egui::Margin::symmetric(8, 2))
                    .corner_radius(10.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(pill_text).size(10.0).color(pill_color));
                    });
            });
        });
    }

    /// Labelled DragValue input row inside a subtle frame.
    fn input_row(
        ui: &mut egui::Ui,
        theme: &theme::ThemeColors,
        label: &str,
        value: &mut u16,
        unit: &str,
        range: std::ops::RangeInclusive<u16>,
    ) {
        Frame::NONE
            .fill(theme.panel_color())
            .stroke(Stroke::new(1.0, theme.accent_color()))
            .inner_margin(egui::Margin::symmetric(10, 6))
            .corner_radius(4.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(label)
                            .size(11.0)
                            .color(theme.text_color().linear_multiply(0.55)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(unit)
                                .size(11.0)
                                .color(theme.text_color().linear_multiply(0.4)),
                        );
                        ui.add(
                            egui::DragValue::new(value)
                                .range(range)
                                .speed(1.0),
                        );
                    });
                });
            });
    }

    /// Metric card showing a single live reading.
    fn reading_card(
        ui: &mut egui::Ui,
        theme: &theme::ThemeColors,
        label: &str,
        value: f32,
        unit: &str,
        stale: bool,
    ) {
        Frame::NONE
            .fill(theme.panel_color())
            .stroke(Stroke::new(1.0, theme.accent_color()))
            .inner_margin(egui::Margin::same(10))
            .corner_radius(4.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(label)
                            .size(10.0)
                            .color(theme.text_color().linear_multiply(0.5)),
                    );
                    ui.add_space(2.0);
                    if stale {
                        ui.label(
                            RichText::new("—")
                                .size(22.0)
                                .color(theme.text_color().linear_multiply(0.25)),
                        );
                        ui.label(
                            RichText::new("stale")
                                .size(10.0)
                                .color(theme.text_color().linear_multiply(0.25)),
                        );
                    } else {
                        ui.label(
                            RichText::new(format!("{:.1}", value))
                                .size(22.0)
                                .color(theme.info_color()),
                        );
                        ui.label(
                            RichText::new(unit)
                                .size(10.0)
                                .color(theme.text_color().linear_multiply(0.4)),
                        );
                    }
                });
            });
    }

    pub fn handle_can_message(&mut self, msg: &messages::MsgFromCan) {
        if std::time::Instant::now().duration_since(self.last_update)
            > std::time::Duration::from_secs(self.timeout_seconds)
        {
            self.is_data_stale = true;
        }

        if let messages::MsgFromCan::ParsedMessage(parsed_msg) = msg {
            if parsed_msg.decoded.msg_id != self.status_msg_id {
                log::info!("Ignoring message with ID: 0x{:X}", parsed_msg.decoded.msg_id);
                return;
            }

            for (_, signal) in parsed_msg.decoded.signals.iter() {
                match signal.name.as_str() {
                    "charge_voltage" => {
                        self.charge_voltage = match &signal.value {
                            can_decode::DecodedSignalValue::Numeric(v) => *v as u16,
                            can_decode::DecodedSignalValue::Enum(v, _) => *v as u16,
                        };
                    }
                    "charge_current" => {
                        self.charge_current = match &signal.value {
                            can_decode::DecodedSignalValue::Numeric(v) => *v as u16,
                            can_decode::DecodedSignalValue::Enum(v, _) => *v as u16,
                        };
                    }
                    "hw_fail" => {
                        self.hardware_fault = match &signal.value {
                            can_decode::DecodedSignalValue::Numeric(v) => *v != 0.0,
                            can_decode::DecodedSignalValue::Enum(v, _) => *v != 0,
                        };
                    }
                    "temp_fail" => {
                        self.temperature_fail = match &signal.value {
                            can_decode::DecodedSignalValue::Numeric(v) => *v != 0.0,
                            can_decode::DecodedSignalValue::Enum(v, _) => *v != 0,
                        };
                    }
                    "input_v_fail" => {
                        self.input_voltage_fault = match &signal.value {
                            can_decode::DecodedSignalValue::Numeric(v) => *v != 0.0,
                            can_decode::DecodedSignalValue::Enum(v, _) => *v != 0,
                        };
                    }
                    "startup_fail" => {
                        self.start_fail = match &signal.value {
                            can_decode::DecodedSignalValue::Numeric(v) => *v != 0.0,
                            can_decode::DecodedSignalValue::Enum(v, _) => *v != 0,
                        };
                    }
                    "communication_fail" => {
                        self.communication_fault = match &signal.value {
                            can_decode::DecodedSignalValue::Numeric(v) => *v != 0.0,
                            can_decode::DecodedSignalValue::Enum(v, _) => *v != 0,
                        };
                    }
                    _ => {}
                }
            }

            self.last_update = std::time::Instant::now();
            self.is_data_stale = false;
        }
    }

    fn send_charge_command(&self, ui_to_can_tx: &std::sync::mpsc::Sender<messages::MsgFromUi>) {
        let voltage_raw = (self.max_charge_voltage as u32 * 10) as u16;
        let current_raw = (self.max_charge_current as u32 * 10) as u16;
        let control: u8 = if self.charge_enable { 0x00 } else { 0x01 };

        let msg_bytes = vec![
            (voltage_raw >> 8) as u8,
            (voltage_raw & 0xFF) as u8,
            (current_raw >> 8) as u8,
            (current_raw & 0xFF) as u8,
            control,
            0x00,
            0x00,
            0x00,
        ];

        let cmd = messages::MsgFromUi::AddSendMessage(messages::AddSendMessage {
            amount: messages::SendAmount::Infinite {
                period: self.command_msg_period,
            },
            msg_id: self.command_msg_id,
            is_msg_id_extended: true,
            msg_bytes,
        });

        let _ = ui_to_can_tx.send(cmd);
    }

    fn stop_charge_command(&self, ui_to_can_tx: &std::sync::mpsc::Sender<messages::MsgFromUi>) { 
        ui_to_can_tx
            .send(messages::MsgFromUi::DeleteSendMessage { msg_id: self.command_msg_id })
            .expect("Failed to send DeleteSendMessage");

    }
}