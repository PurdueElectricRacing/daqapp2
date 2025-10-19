use crate::{can, ui};
use eframe::egui;
// use std::collections::VecDeque;
use hashbrown::HashMap;

pub struct CanViewer {
    pub title: String,
    pub decoded_msgs: HashMap<u32, can::message::ParsedMessage>,
}

impl CanViewer {
    pub fn new(instance_num: usize) -> Self {
        Self {
            title: format!("CAN Viewer #{}", instance_num),
            decoded_msgs: HashMap::new(),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        ui.heading(format!("🚗 {}", self.title));
        ui.separator();

        egui_extras::TableBuilder::new(ui)
            .striped(true)
            .column(egui_extras::Column::auto().at_least(100.0).resizable(true)) // Timestamp
            .column(egui_extras::Column::auto().at_least(300.0).resizable(true)) // Msg (ID)
            .column(egui_extras::Column::remainder().resizable(true)) // Decoded Content
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label("Timestamp");
                });
                header.col(|ui| {
                    ui.label("Msg (ID)");
                });
                header.col(|ui| {
                    ui.label("Decoded Content");
                });
            })
            .body(|mut body| {
                for msg in self.decoded_msgs.values() {
                    body.row(18.0, |mut row| {
                        row.col(|ui| {
                            ui.label(msg.timestamp.format("%H:%M:%S:%3f").to_string());
                        });
                        row.col(|ui| {
                            ui.label(format!("{} (0x{:X})", msg.decoded.name, msg.decoded.msg_id));
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
                }
            });

        ui.scroll_to_cursor(Some(egui::Align::Min));

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
