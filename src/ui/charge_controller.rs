use crate::util;
use crate::{app, messages, theme};
use eframe::egui::{self, Color32, Frame, RichText, Stroke, Vec2};

pub struct ChargeController {
    pub title: String,

    pub max_charge_voltage: f32,
    pub max_charge_current: f32,
    pub charge_enable: bool,
    pub balance_enable: bool,
    // values directly from can msg, is 10x the actual value
    pub charge_voltage_raw: u16, // charge voltage from elcon in decivolts
    pub charge_current_raw: u16, // charge current from elcon in deciamps
    pub is_discharging: bool,

    pub hardware_fault: bool,
    pub temperature_fail: bool,
    pub input_voltage_fault: bool,
    pub start_fail: bool,
    pub communication_fault: bool,

    pub last_update: std::time::Instant,
    pub is_data_stale: bool,
    pub timeout_seconds: u64,

    pub request_msg_period: usize, // default 1 second, 1250 ms stale timeout defined by ABOX
}

impl ChargeController {
    pub fn new() -> Self {
        Self {
            title: "Charge Controller".to_string(),
            max_charge_voltage: 0.0,
            max_charge_current: 0.0,
            charge_enable: false,
            balance_enable: false,
            charge_voltage_raw: 0,
            charge_current_raw: 0,
            is_discharging: false,
            hardware_fault: false,
            temperature_fail: false,
            input_voltage_fault: false,
            start_fail: false,
            communication_fault: false,
            last_update: std::time::Instant::now() - std::time::Duration::from_secs(10), // start as stale
            is_data_stale: true,
            timeout_seconds: 2,
            request_msg_period: 1000,
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        ui_to_can_tx: std::sync::mpsc::Sender<messages::MsgFromUi>,
        parser: Option<&app::ParserInfo>,
    ) -> egui_tiles::UiResponse {
        // Pull theme colors from global ctx storage
        let theme = theme::get_theme(ui.ctx());

        let parser = match Self::check_required_msgs(parser) {
            Ok(parser) => parser,
            Err(e) => {
                ui.add_space(8.0);
                ui.vertical_centered(|ui| {
                    ui.label(format!("DBC error: {e}"));
                    ui.label("CMD+S to toggle the sidebar.");
                    ui.label("Use the sidebar to select a different DBC file");
                });
                return egui_tiles::UiResponse::None;
            }
        };

        self.is_data_stale =
            self.last_update.elapsed() > std::time::Duration::from_secs(self.timeout_seconds);
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
                    .corner_radius(egui::CornerRadius::same(4))
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

                // LEFT: charge request & fault states
                left.vertical(|ui| {

                    Frame::NONE
                        .fill(theme.panel_color())
                        .stroke(Stroke::new(1.0, theme.accent_color()))
                        .inner_margin(egui::Margin::same(10))
                        .corner_radius(egui::CornerRadius::same(4))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("FAULT STATES  (RAW ID 0x18FF50E5 · Byte 5)")
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
                        RichText::new("CHARGE REQUEST")
                            .size(10.0)
                            .color(theme.text_color().linear_multiply(0.5)),
                    );
                    ui.add_space(4.0);
                    ui.add_enabled_ui(!self.charge_enable, |ui| {
                        Self::input_row(ui, &theme, "Max Voltage", &mut self.max_charge_voltage, "V", 0.0..=1000.0);
                        ui.add_space(4.0);
                        Self::input_row(ui, &theme, "Max Current", &mut self.max_charge_current, "A", 0.0..=500.0);
                        ui.add_space(4.0);

                        Self::toggle_button(ui, &theme, "● Balance Enabled", "○ Balance Disabled", &mut self.balance_enable);

                    });

                    ui.add_space(8.0);

                    if Self::toggle_button(ui, &theme, "● Charge Output Enabled", "○ Charge Output Disabled", &mut self.charge_enable) {
                        if self.charge_enable { self.send_charge_request(&ui_to_can_tx, parser); }
                        else { self.stop_charge_request(&ui_to_can_tx, parser); }
                    }

                    ui.add_space(6.0);
                    });

                // RIGHT: Reading cards 
                right.vertical(|ui| {
                    // fault panel
                    ui.label(
                        RichText::new(format!("CHARGER OUTPUT"))
                            .size(10.0)
                            .color(theme.text_color().linear_multiply(0.5)),
                    );
                    ui.add_space(4.0);

                    // show discharging vs charging state from raw current highest bit

                    ui.columns(2, |rcols| {
                        Self::reading_card(
                            &mut rcols[0],
                            &theme,
                            "OUTPUT V",
                            self.charge_voltage_raw as f32 / 10.0,
                            "V",
                            stale,
                        );
                        if self.is_discharging {
                            Self::reading_card(
                                &mut rcols[1],
                                &theme,
                                "DISCHARGING",
                                self.charge_current_raw as f32 / 10.0,
                                "A",
                                stale,
                            );
                        }
                        else {
                            Self::reading_card(
                                &mut rcols[1],
                                &theme,
                                "OUTPUT A",
                                self.charge_current_raw as f32 / 10.0,
                                "A",
                                stale,
                            );
                        }
                    });
                });
            });
        });

        egui_tiles::UiResponse::None
    }
    // helper functions

    fn check_required_msgs(parser: Option<&app::ParserInfo>) -> Result<&app::ParserInfo, String> {
        // check if parser is some
        let parser = parser.ok_or_else(|| "No DBC loaded".to_string())?;

        let msg_defs: Vec<_> = parser.parser.msg_defs().into_iter().collect();

        // charge request message
        let _crm = msg_defs
            .iter()
            .find(|m| m.name == "charge_request")
            .ok_or_else(|| "charge_request not found in DBC".to_string())?;

        // elcon status message
        let _esm = msg_defs
            .iter()
            .find(|m| m.name == "elcon_status")
            .ok_or_else(|| "elcon_status not found in DBC".to_string())?;

        Ok(parser)
    }

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
                    pill_color.r(),
                    pill_color.g(),
                    pill_color.b(),
                    40,
                );

                Frame::NONE
                    .fill(pill_bg)
                    .stroke(Stroke::new(1.0, pill_color.linear_multiply(0.6)))
                    .inner_margin(egui::Margin::symmetric(8, 2))
                    .corner_radius(egui::CornerRadius::same(10))
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
        value: &mut f32,
        unit: &str,
        range: std::ops::RangeInclusive<f32>,
    ) {
        Frame::NONE
            .fill(theme.panel_color())
            .stroke(Stroke::new(1.0, theme.accent_color()))
            .inner_margin(egui::Margin::symmetric(10, 6))
            .corner_radius(egui::CornerRadius::same(4))
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
                                .speed(0.1) // drag in 0.1 steps
                                .fixed_decimals(1), // always show one decimal place
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
            .corner_radius(egui::CornerRadius::same(4))
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

    // balance and charge enable buttons
    fn toggle_button(
        ui: &mut egui::Ui,
        theme: &theme::ThemeColors,
        label_on: &str,
        label_off: &str,
        state: &mut bool,
    ) -> bool {
        let (text, color) = if *state {
            (label_on, theme.success_color())
        } else {
            (label_off, theme.text_color().linear_multiply(0.4))
        };

        let bg = Color32::from_rgba_unmultiplied(
            color.r(),
            color.g(),
            color.b(),
            if *state { 30 } else { 0 },
        );

        let resp = Frame::NONE
            .fill(bg)
            .stroke(Stroke::new(1.0, color))
            .inner_margin(egui::Margin::symmetric(10, 7))
            .corner_radius(egui::CornerRadius::same(4))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.set_max_height(20.0);
                ui.centered_and_justified(|ui| ui.colored_label(color, text));
            })
            .response
            .interact(egui::Sense::click());

        if resp.clicked() {
            *state = !*state;
        }
        resp.clicked()
    }

    pub fn handle_can_message(&mut self, msg: &messages::MsgFromCan) {
        if let messages::MsgFromCan::ParsedMessage(parsed_msg) = msg {
            if parsed_msg.decoded.name == "elcon_status" {
                for (_, signal) in parsed_msg.decoded.signals.iter() {
                    match signal.name.as_str() {
                        "charge_voltage" => {
                            if let can_decode::DecodedSignalValue::Numeric(v) = &signal.value {
                                self.charge_voltage_raw = *v as u16;
                            }
                        }
                        "charge_current" => {
                            if let can_decode::DecodedSignalValue::Numeric(v) = &signal.value {
                                self.charge_current_raw = *v as u16;
                            }
                        }
                        "hw_fail" => {
                            if let can_decode::DecodedSignalValue::Enum(v, _) = &signal.value {
                                self.hardware_fault = *v != 0;
                            }
                        }
                        "temp_fail" => {
                            if let can_decode::DecodedSignalValue::Enum(v, _) = &signal.value {
                                self.temperature_fail = *v != 0;
                            }
                        }
                        "input_v_fail" => {
                            if let can_decode::DecodedSignalValue::Enum(v, _) = &signal.value {
                                self.input_voltage_fault = *v != 0;
                            }
                        }
                        "startup_fail" => {
                            if let can_decode::DecodedSignalValue::Enum(v, _) = &signal.value {
                                self.start_fail = *v != 0;
                            }
                        }
                        "communication_fail" => {
                            if let can_decode::DecodedSignalValue::Enum(v, _) = &signal.value {
                                self.communication_fault = *v != 0;
                            }
                        }
                        _ => {}
                    }
                }

                self.last_update = std::time::Instant::now();
                self.is_data_stale = false;
            }
        }
    }

    fn send_charge_request(
        &self,
        ui_to_can_tx: &std::sync::mpsc::Sender<messages::MsgFromUi>,
        parser: &app::ParserInfo,
    ) {
        // find charge_request message definition
        let crm = parser
            .parser
            .msg_defs()
            .into_iter()
            .find(|m| m.name == "charge_request");

        let crm_id = util::msg_id::can_dbc_to_u32_with_extid_flag(
            &crm.as_ref().expect("Charge request message not found").id,
        );
        let is_extended = matches!(
            &crm.as_ref().expect("Charge request message not found").id,
            can_dbc::MessageId::Extended(_)
        );

        let signal_values: std::collections::HashMap<String, f64> = vec![
            (
                "charge_voltage".to_string(),
                (self.max_charge_voltage as f64),
            ),
            (
                "charge_current".to_string(),
                (self.max_charge_current as f64),
            ),
            ("charge_enable".to_string(), self.charge_enable as u8 as f64),
            (
                "balance_enable".to_string(),
                self.balance_enable as u8 as f64,
            ),
        ]
        .into_iter()
        .collect();

        let Some(msg_bytes) = parser.parser.encode_msg(crm_id, &signal_values) else {
            log::error!("Failed to encode charge_request");
            return;
        };

        ui_to_can_tx
            .send(messages::MsgFromUi::AddSendMessage(
                messages::AddSendMessage {
                    amount: messages::SendAmount::Infinite {
                        period: self.request_msg_period,
                    },
                    msg_id: crm_id,
                    is_msg_id_extended: is_extended,
                    msg_bytes,
                },
            ))
            .expect("Failed to send charge_request");
    }

    fn stop_charge_request(
        &self,
        ui_to_can_tx: &std::sync::mpsc::Sender<messages::MsgFromUi>,
        parser: &app::ParserInfo,
    ) {
        // find charge_request message definition
        let crm = parser
            .parser
            .msg_defs()
            .into_iter()
            .find(|m| m.name == "charge_request");

        let crm_id = util::msg_id::can_dbc_to_u32_with_extid_flag(
            &crm.as_ref().expect("Charge request message not found").id,
        );
        ui_to_can_tx
            .send(messages::MsgFromUi::DeleteSendMessage { msg_id: crm_id })
            .expect("Failed to send DeleteSendMessage");
    }
}
