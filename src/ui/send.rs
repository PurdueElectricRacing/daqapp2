use crate::{app, messages, util};
use eframe::egui;

pub struct SendUi {
    pub title: String,

    search_text: String,
    search_results: Vec<can_dbc::Message>,

    selected_msg: Option<can_dbc::Message>,
    signal_values: Vec<SignalValue>,

    sending_messages: Vec<SendingMessage>,

    send_mode: SendMode,
    period_ms: usize,
    finite_amount: usize,
    adjustable_values_enabled: bool,

    error: Option<String>,

    // Required to be stored on the struct so Drop can send cancellation messages when the UI closes
    ui_to_can_tx: std::sync::mpsc::Sender<messages::MsgFromUi>,
}

#[derive(Clone, Copy, PartialEq)]
enum SendMode {
    Infinite,
    Once,
    Finite,
}

#[derive(Clone)]
struct SignalValue {
    name: String,
    value: f64,
    min: f64,
    max: f64,
}

struct SendingMessage {
    pub amount: messages::SendAmount,
    pub msg_name: String,
    pub msg_id: u32,
    pub msg_id_with_ext_flag: u32,
    pub is_msg_id_extended: bool,
    pub msg_bytes: Vec<u8>,
    pub signal_values: Vec<SignalValue>,
    pub adjustable_values_enabled: bool,
    pub last_sent: chrono::DateTime<chrono::Local>,
}

enum SendUiActions {
    DeleteMessage { msg_id: u32 },
}

impl Drop for SendUi {
    fn drop(&mut self) {
        // When the Send UI is closed, we want to stop all sending messages
        log::info!(
            "Dropping SendUi, stopping all sending messages: {:?}",
            self.sending_messages
                .iter()
                .map(|msg| msg.msg_id)
                .collect::<Vec<_>>()
        );
        for msg in &self.sending_messages {
            let msg_id = msg.msg_id;
            if let Err(e) = self
                .ui_to_can_tx
                .send(messages::MsgFromUi::DeleteSendMessage { msg_id })
            {
                // Don't panic in Drop, just log the error
                log::error!(
                    "Failed to send DeleteSendMessage for msg_id {}: {}",
                    msg_id,
                    e
                );
            }
        }
    }
}

impl SendUi {
    pub fn new(num: usize, ui_to_can_tx: std::sync::mpsc::Sender<messages::MsgFromUi>) -> Self {
        Self {
            title: format!("Send UI {}", num),

            search_text: String::new(),
            search_results: Vec::new(),

            selected_msg: None,
            signal_values: Vec::new(),

            sending_messages: Vec::new(),

            send_mode: SendMode::Infinite,
            period_ms: 1000,
            finite_amount: 10,
            adjustable_values_enabled: false,

            error: None,

            ui_to_can_tx,
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        parser: Option<&app::ParserInfo>,
    ) -> egui_tiles::UiResponse {
        let Some(parser) = parser else {
            ui.vertical_centered(|ui| {
                ui.label("No DBC selected yet.");
                ui.label("CMD+S to toggle the sidebar.");
                ui.label("Use the sidebar to select a DBC file");
            });

            return egui_tiles::UiResponse::None;
        };

        egui::Frame::group(ui.style())
            .inner_margin(egui::Margin::symmetric(8, 6))
            .stroke(egui::Stroke::NONE)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Search:");
                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut self.search_text)
                                    .hint_text("Message name..."),
                            )
                            .changed()
                        {
                            if self.search_text.is_empty() {
                                self.search_results.clear();
                            } else if self.search_text.trim() == "*" {
                                self.search_results = parser.parser.msg_defs().clone();
                            } else {
                                let search_lower = self.search_text.to_lowercase();
                                self.search_results = parser
                                    .parser
                                    .msg_defs()
                                    .iter()
                                    .filter(|msg| {
                                        let id_str = format!("0x{:03X}", util::msg_id::can_dbc_to_u32_without_extid_flag(&msg.id));
                                        id_str.contains(&search_lower) || msg.name.to_lowercase().contains(&search_lower)
                                    })
                                    .cloned()
                                    .collect();
                            }
                        }
                    });

                    ui.add_space(8.0);

                    if self.search_results.is_empty() && !self.search_text.is_empty() {
                        ui.label(egui::RichText::new("No messages found.").italics().weak());
                    } else if self.search_text.is_empty() && self.selected_msg.is_none() {
                        ui.label(
                            egui::RichText::new("Start typing to search for messages... (Use * to show all messages.)")
                                .italics()
                                .weak(),
                        );
                    } else {
                        let mut should_clear_search = false;
                        for msg in &self.search_results {
                            if ui
                                .button(format!(
                                    "{} (0x{:03X})",
                                    msg.name,
                                    util::msg_id::can_dbc_to_u32_without_extid_flag(&msg.id)
                                ))
                                .clicked()
                            {
                                self.selected_msg = Some(msg.clone());
                                self.signal_values = msg
                                    .signals
                                    .iter()
                                    .map(|sig| {
                                        let (min, max) = signal_range(sig);
                                        SignalValue {
                                            name: sig.name.clone(),
                                            value: 0.0,
                                            min,
                                            max,
                                        }
                                    })
                                    .collect();
                                self.error = None;

                                should_clear_search = true;
                            }
                        }
                        if should_clear_search {
                            self.search_text.clear();
                            self.search_results.clear();
                        }
                    }

                    if let Some(selected_msg) = &self.selected_msg {
                        ui.separator();

                        if let Some(error) = &self.error {
                            ui.label(egui::RichText::new(error).color(ui.visuals().error_fg_color));
                        }

                        ui.label(
                            egui::RichText::new(format!(
                                "Selected Message: {} (0x{:03X})",
                                selected_msg.name,
                                util::msg_id::can_dbc_to_u32_without_extid_flag(&selected_msg.id)
                            ))
                            .strong()
                            .size(16.0),
                        );

                        // Send Amount selector
                        ui.label(egui::RichText::new("Send Options").strong());

                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut self.send_mode, SendMode::Once, "Once");
                            ui.selectable_value(
                                &mut self.send_mode,
                                SendMode::Infinite,
                                "Infinite",
                            );
                            ui.selectable_value(&mut self.send_mode, SendMode::Finite, "Finite");
                        });

                        match self.send_mode {
                            SendMode::Once => {}
                            SendMode::Infinite => {
                                ui.horizontal(|ui| {
                                    ui.label("Period (ms)");
                                    ui.add(
                                        egui::DragValue::new(&mut self.period_ms)
                                            .speed(1)
                                            .range(1..=10_000),
                                    );
                                });
                            }
                            SendMode::Finite => {
                                ui.horizontal(|ui| {
                                    ui.label("Amount");
                                    ui.add(
                                        egui::DragValue::new(&mut self.finite_amount)
                                            .speed(1)
                                            .range(1..=10_000),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Period (ms)");
                                    ui.add(
                                        egui::DragValue::new(&mut self.period_ms)
                                            .speed(1)
                                            .range(1..=10_000),
                                    );
                                });
                            }
                        }
                        ui.checkbox(&mut self.adjustable_values_enabled, "Adjustable values");
                        for i in 0..self.signal_values.len() {
                            ui.horizontal(|ui| {
                                let signal = &mut self.signal_values[i];
                                ui.label(signal.name.as_str());
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut signal.value)
                                        .range(signal.min..=signal.max)
                                        .speed(0.1),
                                    )
                                    .changed()
                                {
                                    self.signal_values[i].value = signal.value;
                                }
                            });
                        }

                        if ui.button("Send Message").clicked() {
                            let msg_id_with_ext_flag =
                                util::msg_id::can_dbc_to_u32_with_extid_flag(&selected_msg.id);
                            let encoded =
                                encode_msg_from_signals(&parser.parser, msg_id_with_ext_flag, &self.signal_values);

                            let Some(msg_bytes) = encoded else {
                                self.error = Some(
                                    "Failed to encode message. Check signal values.".to_string(),
                                );

                                return;
                            };

                            self.error = None;

                            let send_amount = match self.send_mode {
                                SendMode::Once => messages::SendAmount::Once,

                                SendMode::Infinite => messages::SendAmount::Infinite {
                                    period: self.period_ms,
                                },

                                SendMode::Finite => messages::SendAmount::Finite {
                                    amount: self.finite_amount,
                                    period: self.period_ms,
                                },
                            };

                            let msg_id_u32 =
                                util::msg_id::can_dbc_to_u32_without_extid_flag(&selected_msg.id);

                            self.sending_messages.push(SendingMessage {
                                amount: send_amount,
                                msg_name: selected_msg.name.clone(),
                                msg_id: msg_id_u32,
                                msg_id_with_ext_flag,
                                is_msg_id_extended: matches!(
                                    selected_msg.id,
                                    can_dbc::MessageId::Extended(_)
                                ),
                                msg_bytes: msg_bytes.clone(),
                                signal_values: self.signal_values.clone(),
                                adjustable_values_enabled: self.adjustable_values_enabled,
                                last_sent: chrono::Local::now(),
                            });

                            let add_send_msg = messages::AddSendMessage {
                                amount: send_amount,
                                msg_id: msg_id_u32,
                                is_msg_id_extended: matches!(selected_msg.id, can_dbc::MessageId::Extended(_)),
                                msg_bytes,
                            };

                            self.selected_msg = None;
                            self.signal_values.clear();
                            self.adjustable_values_enabled = false;

                            self.ui_to_can_tx
                                .send(messages::MsgFromUi::AddSendMessage(add_send_msg))
                                .expect("Failed to send AddSendMessage");
                        }
                    }

                    ui.separator();

                    let mut all_actions = Vec::new();
                    let mut updates_to_send = Vec::new();
                    for idx in (0..self.sending_messages.len()).rev() {
                        if let Some(action) =
                            self.sending_messages[idx].ui(ui, idx, &mut updates_to_send)
                        {
                            all_actions.push(action);
                        }
                        ui.add_space(8.0);
                    }

                    updates_to_send.sort_unstable();
                    updates_to_send.dedup();
                    for idx in updates_to_send {
                        if idx >= self.sending_messages.len() {
                            continue;
                        }

                        let msg = &self.sending_messages[idx];
                        if !msg.adjustable_values_enabled {
                            continue;
                        }
                        let encoded = encode_msg_from_signals(
                            &parser.parser,
                            msg.msg_id_with_ext_flag,
                            &msg.signal_values,
                        );
                        let Some(msg_bytes) = encoded else {
                            self.error = Some(format!(
                                "Failed to encode {} while applying slider update.",
                                msg.msg_name
                            ));
                            continue;
                        };

                        self.sending_messages[idx].msg_bytes = msg_bytes.clone();
                        self.error = None;

                        self.ui_to_can_tx
                            .send(messages::MsgFromUi::AddSendMessage(messages::AddSendMessage {
                                amount: self.sending_messages[idx].amount,
                                msg_id: self.sending_messages[idx].msg_id,
                                is_msg_id_extended: self.sending_messages[idx].is_msg_id_extended,
                                msg_bytes,
                            }))
                            .expect("Failed to send AddSendMessage");
                    }

                    for action in all_actions {
                        match action {
                            SendUiActions::DeleteMessage { msg_id } => {
                                self.sending_messages.retain(|msg| msg.msg_id != msg_id);
                                self.ui_to_can_tx
                                    .send(messages::MsgFromUi::DeleteSendMessage { msg_id })
                                    .expect("Failed to send DeleteSendMessage");
                            }
                        }
                    }
                });
            });

        egui_tiles::UiResponse::None
    }

    pub fn handle_can_message(&mut self, msg: &messages::MsgFromCan) {
        if let messages::MsgFromCan::MessageSent {
            msg_id,
            timestamp,
            amount_left,
        } = msg
        {
            if let Some(rx_amount_left) = amount_left {
                for sending_msg in &mut self.sending_messages {
                    if sending_msg.msg_id == *msg_id {
                        sending_msg.last_sent = *timestamp;
                        sending_msg.amount = *rx_amount_left;
                        break;
                    }
                }
            } else {
                // If amount_left is None, it means the message is done sending,
                // so we remove it from the list
                self.sending_messages.retain(|msg| msg.msg_id != *msg_id);
            }
        }
    }
}

impl SendingMessage {
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        msg_idx: usize,
        updates_to_send: &mut Vec<usize>,
    ) -> Option<SendUiActions> {
        let mut delete_action = None;
        let raw_bytes_str = self
            .msg_bytes
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        // Header (outside card)
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{}  (0x{:03X})", self.msg_name, self.msg_id))
                    .strong()
                    .size(16.0)
                    .color(ui.visuals().text_color()),
            );
            ui.label(
                egui::RichText::new(self.amount.display()).color(ui.visuals().text_color()),
            );
            ui.label(
                egui::RichText::new(format!(
                    "~{} ms ago",
                    (chrono::Local::now() - self.last_sent).num_milliseconds()
                ))
                    .italics()
                    .color(ui.visuals().weak_text_color()),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🗑").on_hover_text("Delete message").clicked() {
                    delete_action = Some(SendUiActions::DeleteMessage {
                        msg_id: self.msg_id,
                    });
                }
                ui.label(
                    egui::RichText::new(raw_bytes_str)
                        .monospace()
                        .color(ui.visuals().text_color()),
                );

                ui.add_space(2.0);
            });
        });

        ui.add_space(4.0);

        // Card container
        egui::Frame::group(ui.style())
            .fill(ui.visuals().faint_bg_color)
            .corner_radius(egui::CornerRadius::same(8))
            .inner_margin(egui::Margin::symmetric(8, 6))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    let total_signals = self.signal_values.len();
                    for (i, signal) in self.signal_values.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(&signal.name)
                                    .monospace()
                                    .color(ui.visuals().text_color()),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if self.adjustable_values_enabled {
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut signal.value)
                                                    .range(signal.min..=signal.max)
                                                    .speed(0.1),
                                            )
                                            .changed()
                                        {
                                            updates_to_send.push(msg_idx);
                                        }
                                    } else {
                                        ui.label(
                                            egui::RichText::new(format!("{:.2}", signal.value))
                                                .monospace(),
                                        );
                                    }
                                },
                            );
                        });
                        if i < total_signals - 1 {
                            ui.separator();
                        }
                    }
                });
            });

        delete_action
    }
}

fn encode_msg_from_signals(
    parser: &can_decode::Parser,
    msg_id_with_ext_flag: u32,
    signals: &[SignalValue],
) -> Option<Vec<u8>> {
    let values_hashmap = signals
        .iter()
        .map(|signal| (signal.name.clone(), signal.value))
        .collect();
    parser.encode_msg(msg_id_with_ext_flag, &values_hashmap)
}

fn signal_range(sig: &can_dbc::Signal) -> (f64, f64) {
    let fallback = (-1000.0, 1000.0);
    let mut min = sig.min;
    let mut max = sig.max;

    if !min.is_finite() || !max.is_finite() || min >= max {
        min = fallback.0;
        max = fallback.1;
    }

    (min, max)
}
