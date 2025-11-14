use crate::can;
use eframe::egui;
use hashbrown::HashMap;

pub struct ViewerTable {
    pub title: String,
    pub decoded_msgs: HashMap<u32, can::message::ParsedMessage>,
    pub frozen_msgs: Option<HashMap<u32, can::message::ParsedMessage>>,
    pub paused: bool,
    pub search: String,
}

impl ViewerTable {
    pub fn new(instance_num: usize) -> Self {
        Self {
            title: format!("CAN Viewer Table #{}", instance_num),
            decoded_msgs: HashMap::new(),
            frozen_msgs: None,
            paused: false,
            search: String::new(),
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        pending_scope_spawns: &mut Vec<(u32, String, String)>,
    ) -> egui_tiles::UiResponse {
        ui.heading(format!("🚗 {}", self.title));
        if ui
            .button(if self.paused { "Resume" } else { "Pause" })
            .clicked()
        {
            self.paused = !self.paused;
            if self.paused {
                self.frozen_msgs = Some(self.decoded_msgs.clone());
            } else {
                self.frozen_msgs = None;
            }
        }

        ui.separator();

        ui.add_space(4.0);

        egui::Frame::group(ui.style())
            .inner_margin(egui::Margin::symmetric(8, 6))
            .stroke(egui::Stroke::NONE)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.text_edit_singleline(&mut self.search);
                });
                ui.add_space(8.0);

                let msgs = if let Some(frozen) = &self.frozen_msgs {
                    frozen
                } else {
                    &self.decoded_msgs
                };

                if msgs.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new("No CAN messages to display.")
                                .italics()
                                .weak(),
                        );
                    });
                    return;
                }

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let low_search = self.search.to_lowercase();
                    let mut msg_keys = msgs
                        .iter()
                        .filter_map(|(&msg_id, msg)| {
                            if self.search.is_empty()
                                || msg.decoded.name.to_lowercase().contains(&low_search)
                                || msg
                                    .decoded
                                    .msg_id
                                    .to_string()
                                    .to_lowercase()
                                    .contains(&low_search)
                                || msg.tx_node.to_lowercase().contains(&low_search)
                                || msg
                                    .decoded
                                    .signals
                                    .values()
                                    .any(|sig| sig.name.to_lowercase().contains(&low_search))
                            {
                                Some(msg_id)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();
                    msg_keys.sort();
                    for msg_id in msg_keys {
                        let msg = &msgs[&msg_id];
                        let mut signal_keys =
                            msg.decoded.signals.keys().cloned().collect::<Vec<_>>();
                        signal_keys.sort();
                        let signals: Vec<(&str, String)> = signal_keys
                            .iter()
                            .map(|sig_name| {
                                let signal = &msg.decoded.signals[sig_name];
                                if signal.unit.is_empty() {
                                    (signal.name.as_str(), format!("{}", signal.value))
                                } else {
                                    (
                                        signal.name.as_str(),
                                        format!("{} {}", signal.value, signal.unit),
                                    )
                                }
                            })
                            .collect();
                        let raw_bytes_str = msg
                            .raw_bytes
                            .iter()
                            .map(|b| format!("{:02X}", b))
                            .collect::<Vec<_>>()
                            .join(" ");
                        message_card(
                            ui,
                            &msg.decoded.name,
                            msg.decoded.msg_id,
                            msg.tx_node.as_str(),
                            raw_bytes_str.as_str(),
                            &msg.timestamp.format("%-I:%M:%S%.3f").to_string(),
                            &signals,
                            &self.search,
                            pending_scope_spawns,
                        );
                        ui.add_space(8.0);
                    }
                });
            });

        egui_tiles::UiResponse::None
    }

    pub fn handle_can_message(&mut self, msg: &can::can_messages::CanMessage) {
        match msg {
            can::can_messages::CanMessage::ParsedMessage(parsed_msg) => {
                self.decoded_msgs
                    .insert(parsed_msg.decoded.msg_id, parsed_msg.clone());
            }
            _ => {}
        }
    }
}

fn message_card(
    ui: &mut egui::Ui,
    msg_name: &str,
    msg_id: u32,
    tx_node: &str,
    raw_bytes: &str,
    timestamp: &str,
    signals: &[(&str, String)],
    search: &str,
    pending_scope_spawns: &mut Vec<(u32, String, String)>,
) {
    // Header (outside card)
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("{}  (0x{:03X})", msg_name, msg_id))
                .strong()
                .size(16.0)
                .color(
                    if search.is_empty() || msg_name.to_lowercase().contains(&search.to_lowercase())
                    {
                        ui.visuals().text_color()
                    } else {
                        ui.visuals().weak_text_color()
                    },
                ),
        );
        ui.label(egui::RichText::new(format!("from {}", tx_node)).color(
            if search.is_empty() || tx_node.to_lowercase().contains(&search.to_lowercase()) {
                ui.visuals().text_color()
            } else {
                ui.visuals().weak_text_color()
            },
        ));
        ui.label(
            egui::RichText::new(timestamp)
                .italics()
                .color(ui.visuals().weak_text_color()),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(raw_bytes)
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
                for (i, (sig_name, value)) in signals.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(*sig_name).monospace().color(
                            if search.is_empty()
                                || sig_name.to_lowercase().contains(&search.to_lowercase())
                            {
                                ui.visuals().text_color()
                            } else {
                                ui.visuals().weak_text_color()
                            },
                        ));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("📊").clicked() {
                                pending_scope_spawns.push((
                                    msg_id,
                                    msg_name.to_string(),
                                    sig_name.to_string(),
                                ));
                            }
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new(value).monospace());
                            if ui.small_button("add scope").clicked() {
                                pending_scope_spawns.push((msg_id, sig_name.to_string()));
                            }
                        });
                    });
                    if i < signals.len() - 1 {
                        ui.separator();
                    }
                }
            });
        });
}
