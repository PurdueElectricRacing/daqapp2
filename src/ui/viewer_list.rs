use crate::messages;
use eframe::egui;
use std::collections::VecDeque;

const MAX_MESSAGES: usize = 200;

pub struct ViewerList {
    pub title: String,
    pub decoded_msgs: VecDeque<messages::ParsedMessage>,
    pub frozen_msgs: Option<VecDeque<messages::ParsedMessage>>,
    pub paused: bool,
}

impl ViewerList {
    pub fn new(instance_num: usize) -> Self {
        Self {
            title: format!("CAN Viewer Table #{}", instance_num),
            decoded_msgs: VecDeque::new(),
            frozen_msgs: None,
            paused: false,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
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

        egui_extras::TableBuilder::new(ui)
            .striped(true)
            .column(egui_extras::Column::auto().at_least(100.0).resizable(true)) // Timestamp
            .column(egui_extras::Column::auto().at_least(300.0).resizable(true)) // Msg (ID)
            .column(egui_extras::Column::auto().at_least(200.0).resizable(true)) // Signal
            .column(egui_extras::Column::remainder().resizable(true)) // Decoded Signal
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label("Timestamp");
                });
                header.col(|ui| {
                    ui.label("Msg (ID)");
                });
                header.col(|ui| {
                    ui.label("Signal");
                });
                header.col(|ui| {
                    ui.label("Decoded Content");
                });
            })
            .body(|mut body| {
                let msgs = if let Some(frozen) = &self.frozen_msgs {
                    frozen
                } else {
                    &self.decoded_msgs
                };
                for msg in msgs.iter().rev() {
                    for (sig_name, signal) in msg.decoded.signals.iter() {
                        body.row(18.0, |mut row| {
                            row.col(|ui| {
                                ui.label(msg.timestamp.format("%H:%M:%S:%3f").to_string());
                            });
                            row.col(|ui| {
                                ui.label(format!(
                                    "{} (0x{:X})",
                                    msg.decoded.name, msg.decoded.msg_id
                                ));
                            });
                            row.col(|ui| {
                                ui.label(sig_name.to_string());
                            });
                            row.col(|ui| {
                                if signal.unit.is_empty() {
                                    ui.label(format!("{:.2}", signal.value));
                                } else {
                                    ui.label(format!("{:.2} {}", signal.value, signal.unit));
                                }
                            });
                        });
                    }
                }
            });

        ui.scroll_to_cursor(Some(egui::Align::Min));

        egui_tiles::UiResponse::None
    }

    pub fn handle_can_message(&mut self, msg: &messages::ParsedMessage) {
        if self.paused {
            return;
        }

        while self.decoded_msgs.len() >= MAX_MESSAGES - 1 {
            self.decoded_msgs.pop_front();
        }

        self.decoded_msgs.push_back(msg.clone());
    }
}
