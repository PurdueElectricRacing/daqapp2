use crate::can;
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
                // rows is more performant than iter + row
                body.rows(18.0, self.decoded_msgs.len(), |mut row| {
                    let msg = &self.decoded_msgs[row.index()];
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
}
