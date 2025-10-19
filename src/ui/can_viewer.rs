use crate::{can, ui};
use eframe::egui;
use std::collections::VecDeque;

const CAN_VIEWER_MAX_DECODED_MSGS: usize = 1000; // TODO: have a scroll back -> query db

pub struct CanViewer {
    pub title: String,
    pub decoded_msgs: VecDeque<can::message::ParsedMessage>,
}

impl CanViewer {
    pub fn new(instance_num: usize) -> Self {
        Self {
            title: format!("CAN Viewer #{}", instance_num),
            decoded_msgs: VecDeque::with_capacity(CAN_VIEWER_MAX_DECODED_MSGS),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        ui.heading(format!("🚗 {}", self.title));
        ui.separator();

        egui_extras::TableBuilder::new(ui)
            .striped(true)
            .column(egui_extras::Column::auto().at_least(100.0).resizable(true)) // Timestamp
            .column(egui_extras::Column::auto().at_least(100.0).resizable(true)) // ID
            .column(egui_extras::Column::remainder().resizable(true)) // Decoded Content
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label("Timestamp");
                });
                header.col(|ui| {
                    ui.label("ID");
                });
                header.col(|ui| {
                    ui.label("Decoded Content");
                });
            })
            .body(|body| {
                let row_count = self.decoded_msgs.len();
                // rows is more performant than iter + row
                body.rows(18.0, row_count, |mut row| {
                    let idx = row_count - row.index() - 1; // Reverse order
                    let msg = &self.decoded_msgs[idx];
                    row.col(|ui| {
                        ui.label(msg.timestamp.format("%H:%M:%S:%3f").to_string());
                    });
                    row.col(|ui| {
                        ui.label(format!("0x{:X}", msg.decoded.msg_id));
                    });
                    row.col(|ui| {
                        let signals_str = msg
                            .decoded
                            .signals
                            .iter()
                            .map(|(name, sig)| format!("{}: {} {}", name, sig.value, sig.unit))
                            .collect::<Vec<String>>()
                            .join(", ");
                        ui.label(signals_str.to_string());
                    });
                });
            });

        egui_tiles::UiResponse::None
    }

    pub fn handle_can_message(&mut self, msg: &can::can_messages::CanMessage) {
        match msg {
            can::can_messages::CanMessage::ParsedMessage(parsed_msg) => {
                if self.decoded_msgs.len() == CAN_VIEWER_MAX_DECODED_MSGS {
                    self.decoded_msgs.pop_front();
                }
                self.decoded_msgs.push_back(parsed_msg.clone());
            }
            _ => {}
        }
    }
}
