use crate::{action, can};
use eframe::egui;

type MsgMap = hashbrown::HashMap<u32, can::message::ParsedMessage>;

pub struct ViewerTable {
    pub title: String,
    pub decoded_msgs: MsgMap,
    pub frozen_msgs: Option<MsgMap>,
    pub paused: bool,
    pub search: String,
}

impl ViewerTable {
    pub fn new(instance_num: usize) -> Self {
        Self {
            title: format!("CAN Viewer Table #{}", instance_num),
            decoded_msgs: MsgMap::new(),
            frozen_msgs: None,
            paused: false,
            search: String::new(),
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        action_queue: &mut Vec<action::AppAction>,
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
                                || msg.decoded.tx_node.to_lowercase().contains(&low_search)
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
                        let signals: Vec<(&str, String)> = msg
                            .decoded
                            .signals
                            .iter()
                            .map(|(sig_name, signal)| {
                                if signal.unit.is_empty() {
                                    (sig_name.as_str(), format!("{:.2}", signal.value))
                                } else {
                                    (
                                        sig_name.as_str(),
                                        format!("{:.2} {}", signal.value, signal.unit),
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
                        MessageCard {
                            msg_name: &msg.decoded.name,
                            msg_id: msg.decoded.msg_id,
                            tx_node: &msg.decoded.tx_node,
                            raw_bytes: &raw_bytes_str,
                            timestamp: &msg.timestamp.format("%-I:%M:%S%.3f").to_string(),
                            signals,
                            search: &self.search,
                        }
                        .ui(ui)
                        .into_iter()
                        .for_each(|spawn| action_queue.push(spawn));
                        ui.add_space(8.0);
                    }
                });
            });

        egui_tiles::UiResponse::None
    }

    pub fn handle_can_message(&mut self, msg: &can::message::ParsedMessage) {
        self.decoded_msgs.insert(msg.decoded.msg_id, msg.clone());
    }
}

struct MessageCard<'a> {
    msg_name: &'a str,
    msg_id: u32,
    tx_node: &'a str,
    raw_bytes: &'a str,
    timestamp: &'a str,
    signals: Vec<(&'a str, String)>,
    search: &'a str,
}

impl MessageCard<'_> {
    fn ui(&self, ui: &mut egui::Ui) -> Vec<action::AppAction> {
        let mut action_queue = Vec::new();
        // Header (outside card)
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{}  (0x{:03X})", self.msg_name, self.msg_id))
                    .strong()
                    .size(16.0)
                    .color(
                        if self.search.is_empty()
                            || self
                                .msg_name
                                .to_lowercase()
                                .contains(&self.search.to_lowercase())
                        {
                            ui.visuals().text_color()
                        } else {
                            ui.visuals().weak_text_color()
                        },
                    ),
            );
            ui.label(
                egui::RichText::new(format!("from {}", self.tx_node)).color(
                    if self.search.is_empty()
                        || self
                            .tx_node
                            .to_lowercase()
                            .contains(&self.search.to_lowercase())
                    {
                        ui.visuals().text_color()
                    } else {
                        ui.visuals().weak_text_color()
                    },
                ),
            );
            ui.label(
                egui::RichText::new(self.timestamp)
                    .italics()
                    .color(ui.visuals().weak_text_color()),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(self.raw_bytes)
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
                    for (i, (sig_name, value)) in self.signals.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(*sig_name).monospace().color(
                                    if self.search.is_empty()
                                        || sig_name
                                            .to_lowercase()
                                            .contains(&self.search.to_lowercase())
                                    {
                                        ui.visuals().text_color()
                                    } else {
                                        ui.visuals().weak_text_color()
                                    },
                                ),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.small_button("📊").clicked() {
                                        action_queue.push(action::AppAction::SpawnWidget(
                                            action::WidgetType::Scope {
                                                msg_id: self.msg_id,
                                                msg_name: self.msg_name.to_string(),
                                                signal_name: sig_name.to_string(),
                                            },
                                        ));
                                    }
                                    ui.add_space(8.0);
                                    ui.label(egui::RichText::new(value).monospace());
                                },
                            );
                        });
                        if i < self.signals.len() - 1 {
                            ui.separator();
                        }
                    }
                });
            });

        action_queue
    }
}
