use crate::util;
use crate::{app, messages, theme};
use eframe::egui::{self, Color32, Frame, RichText, Stroke, Vec2};
use egui_plot::{Line, Plot, PlotPoints};

pub struct ChargeController {
    pub title: String,

    pub max_charge_voltage: f32,
    pub max_charge_current: f32,
    pub charge_enable: bool,

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

    pub status_msg_id: u32,
    pub charge_request_msg: Option<can_dbc::Message>,
    pub charge_request_msg_id: u32,

    pub request_msg_period: usize, // default 1 second, 1250 ms stale timeout defined by ABOX

    // abox reported telemetry
    pub charging_telemetry_msg: Option<can_dbc::Message>,
    pub charging_telemetry_msg_id: u32,
    pub pack_voltage: f32,
    pub min_cell_voltage: f32,
    pub max_cell_voltage: f32,
    pub charging_state: u8,

    pub voltage_history: std::collections::VecDeque<[f64; 2]>, // [time, value]
    pub current_history: std::collections::VecDeque<[f64; 2]>,
    pub start_time: std::time::Instant,

    pub max_history: usize, // ~5 min at 1hz
}

impl ChargeController {
    pub fn new() -> Self {
        Self {
            title: "Charge Controller".to_string(),
            max_charge_voltage: 0.0,
            max_charge_current: 0.0,
            charge_enable: false,
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
            status_msg_id: 0x98FF50E5, // for some reason definition in dbc is eid? override has correct number but idk
            request_msg_period: 1000,
            charge_request_msg: None,
            charge_request_msg_id: 0,
            charging_telemetry_msg: None,
            charging_telemetry_msg_id: 0,
            pack_voltage: 0.0,
            min_cell_voltage: 0.0,
            max_cell_voltage: 0.0,
            charging_state: 0,
            voltage_history: std::collections::VecDeque::new(),
            current_history: std::collections::VecDeque::new(),
            start_time: std::time::Instant::now(),
            max_history: 300, // ~5 min at 1hz
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

        let Some(parser) = parser else {
            ui.add_space(8.0);
            ui.vertical_centered(|ui| {
                ui.label("No DBC selected yet.");
                ui.label("CMD+S to toggle the sidebar.");
                ui.label("Use the sidebar to select a DBC file");
            });

            return egui_tiles::UiResponse::None;
        };

        let Some(charge_request_msg) = parser
            .parser
            .msg_defs()
            .into_iter()
            .find(|m| m.name == "charge_request")
        else {
            log::error!("charge request message not found in dbc");
            ui.add_space(8.0);
            ui.vertical_centered(|ui| {
                ui.label("DBC does not contain charge request message definition.");
                ui.label("CMD+S to toggle the sidebar.");
                ui.label("Use the sidebar to select a different DBC file");
            });
            return egui_tiles::UiResponse::None;
        };

        let Some(charging_telemetry_msg) = parser
            .parser
            .msg_defs()
            .into_iter()
            .find(|m| m.name == "charging_telemetry")
        else {
            log::error!("charging telemetry message not found in dbc");
            ui.add_space(8.0);
            ui.vertical_centered(|ui| {
                ui.label("DBC does not contain charging telemetry message definition.");
                ui.label("CMD+S to toggle the sidebar.");
                ui.label("Use the sidebar to select a different DBC file");
            });
            return egui_tiles::UiResponse::None;
        };

        self.charge_request_msg = Some(charge_request_msg.clone());
        self.charge_request_msg_id =
            util::msg_id::can_dbc_to_u32_with_extid_flag(&charge_request_msg.id);

        self.charging_telemetry_msg = Some(charging_telemetry_msg.clone());
        self.charging_telemetry_msg_id =
            util::msg_id::can_dbc_to_u32_with_extid_flag(&charging_telemetry_msg.id);

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

                // LEFT: fault states & charge command 
                left.vertical(|ui| {
                    // fault panel
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
                        ui.add_space(8.0);

                    });

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
                        .corner_radius(egui::CornerRadius::same(4))
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
                            self.send_charge_request(&ui_to_can_tx, parser);
                        }
                        else {
                            self.stop_charge_request(&ui_to_can_tx);
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

                    ui.add_space(10.0);
                    // telemetry panel with voltage and current plots
                    Frame::NONE
                        .fill(theme.panel_color())
                        .stroke(Stroke::new(1.0, theme.accent_color()))
                        .inner_margin(egui::Margin::same(10))
                        .corner_radius(egui::CornerRadius::same(4))
                        .show(ui, |ui| {
                            ui.set_min_height(120.0);
                            ui.set_min_width(ui.available_width());

                    // State badge
                    let state_label = if self.charging_state == 1 { "● CHARGING" } else { "○ IDLE" };
                    let state_color = if self.charging_state == 1 { theme.success_color() } else { theme.text_color().linear_multiply(0.4) };
                    ui.horizontal(|ui| {
                        ui.colored_label(state_color, state_label);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(format!(
                                "cell {:.2}–{:.2} V",
                                self.min_cell_voltage, self.max_cell_voltage
                            )).size(10.0).color(theme.text_color().linear_multiply(0.5)));
                        });
                    });

                    ui.add_space(4.0);

                    Plot::new("charge_plot")
                        .height(200.0)
                        .show_axes([false, true])
                        .show_grid(false)
                        .allow_drag(false)
                        .allow_zoom(false)
                        .include_y(0.0)
                        .show(ui, |plot_ui: &mut egui_plot::PlotUi| {
                            let v_points = PlotPoints::new(self.voltage_history.iter().cloned().collect());
                            plot_ui.line(Line::new("v_points",v_points)
                                .color(theme.info_color())
                                .name("Pack V"));

                            let i_points = PlotPoints::new(self.current_history.iter().cloned().collect());
                            plot_ui.line(Line::new("i_points",i_points)
                                .color(theme.warning_color())
                                .name("Output A"));
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

    pub fn handle_can_message(&mut self, msg: &messages::MsgFromCan) {
        if std::time::Instant::now().duration_since(self.last_update)
            > std::time::Duration::from_secs(self.timeout_seconds)
        {
            self.is_data_stale = true;
        }

        if let messages::MsgFromCan::ParsedMessage(parsed_msg) = msg {
            if parsed_msg.decoded.msg_id == self.status_msg_id {
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

                self.is_discharging = if self.charge_current_raw == u16::MAX {true} else {false};
                log::info!("is_discharging: {}, raw_current: {}", self.is_discharging, self.charge_current_raw);

                self.last_update = std::time::Instant::now();
                self.is_data_stale = false;
            }
            if parsed_msg.decoded.msg_id == self.charging_telemetry_msg_id {
                for (_, signal) in parsed_msg.decoded.signals.iter() {
                    match signal.name.as_str() {
                        "pack_voltage" => {
                            if let can_decode::DecodedSignalValue::Numeric(v) = &signal.value {
                                self.pack_voltage = *v as f32;
                            }
                        }
                        "min_cell_voltage" => {
                            if let can_decode::DecodedSignalValue::Numeric(v) = &signal.value {
                                self.min_cell_voltage = *v as f32;
                            }
                        }
                        "max_cell_voltage" => {
                            if let can_decode::DecodedSignalValue::Numeric(v) = &signal.value {
                                self.max_cell_voltage = *v as f32;
                            }
                        }
                        "charging_state" => {
                            if let can_decode::DecodedSignalValue::Numeric(v) = &signal.value {
                                self.charging_state = *v as u8;
                            }
                        }
                        _ => {}
                    }
                }
                let t = self.start_time.elapsed().as_secs_f64();
                self.voltage_history
                    .push_back([t, self.pack_voltage as f64]);
                self.current_history
                    .push_back([t, self.charge_current_raw as f64 / 10.0]);
                if self.voltage_history.len() > self.max_history {
                    self.voltage_history.pop_front();
                }
                if self.current_history.len() > self.max_history {
                    self.current_history.pop_front();
                }
            }
        }
    }

    fn send_charge_request(
        &self,
        ui_to_can_tx: &std::sync::mpsc::Sender<messages::MsgFromUi>,
        parser: &app::ParserInfo,
    ) {
        let is_extended = matches!(
            self.charge_request_msg
                .as_ref()
                .expect("Charge request message not found")
                .id,
            can_dbc::MessageId::Extended(_)
        );

        let signal_values: std::collections::HashMap<String, f64> = vec![
            ("charge_enable".to_string(), self.charge_enable as u8 as f64),
            (
                "charge_voltage".to_string(),
                (self.max_charge_voltage * 10.0) as f64,
            ),
            (
                "charge_current".to_string(),
                (self.max_charge_current * 10.0) as f64,
            ),
        ]
        .into_iter()
        .collect();

        let Some(msg_bytes) = parser
            .parser
            .encode_msg(self.charge_request_msg_id, &signal_values)
        else {
            log::error!("Failed to encode charge_request");
            log::error!(
                "Attempting encode with msg_id: {} (0x{:X})",
                self.charge_request_msg_id,
                self.charge_request_msg_id
            );
            log::error!(
                "Charge request message definition: {:?}",
                self.charge_request_msg
            );
            return;
        };

        ui_to_can_tx
            .send(messages::MsgFromUi::AddSendMessage(
                messages::AddSendMessage {
                    amount: messages::SendAmount::Infinite {
                        period: self.request_msg_period,
                    },
                    msg_id: self.charge_request_msg_id,
                    is_msg_id_extended: is_extended,
                    msg_bytes,
                },
            ))
            .expect("Failed to send charge_request");
    }

    fn stop_charge_request(&self, ui_to_can_tx: &std::sync::mpsc::Sender<messages::MsgFromUi>) {
        ui_to_can_tx
            .send(messages::MsgFromUi::DeleteSendMessage {
                msg_id: self.charge_request_msg_id,
            })
            .expect("Failed to send DeleteSendMessage");
    }
}
