use crate::ui;
use chrono::Timelike;
use eframe::egui;

struct SendMessage {
    msg_id: u32,
    msg_id_extended: bool,
    msg_bytes: Vec<u8>,
    period: i32, // milliseconds (negative = means ignore)
    last_sent_time: chrono::DateTime<chrono::Utc>,
}

pub struct Send {
    pub title: String,
    sending: Vec<SendMessage>,
    is_paused: bool,
    current_msg_id: String,
    current_data: String,
    current_period: String,
    current_extended: bool,
}

impl Send {
    pub fn new(instance_num: usize) -> Self {
        let title = format!("Send #{}", instance_num);
        Self {
            title,
            sending: Vec::new(),
            is_paused: false,
            current_msg_id: String::new(),
            current_data: String::new(),
            current_period: String::new(),
            current_extended: false,
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        ui_sender: &std::sync::mpsc::Sender<ui::ui_messages::UiMessage>,
    ) -> egui_tiles::UiResponse {
        ui.heading(format!("{}", self.title));

        // Pause/Resume button
        let pause_text = if self.is_paused {
            "▶ Resume"
        } else {
            "⏸ Pause"
        };
        if ui.button(pause_text).clicked() {
            self.is_paused = !self.is_paused;
        }

        ui.separator();

        // Current sending info
        if self.sending.is_empty() {
            ui.label("No messages being sent.");
        } else {
            ui.label(format!("Sending {} messages", self.sending.len()));
            let now = chrono::Utc::now();
            for send_msg in &mut self.sending {
                if send_msg.period < 0 {
                    continue;
                }
                let raw_bytes_str = send_msg
                    .msg_bytes
                    .iter()
                    .map(|b| format!("{:02X}", b))
                    .collect::<Vec<_>>()
                    .join(" ");
                let elapsed = now.signed_duration_since(send_msg.last_sent_time);
                let ms_ago = elapsed.num_milliseconds();
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "ID: {:X} ({}), Data: {} every {} ms. Last sent ~{} ms ago",
                        send_msg.msg_id, send_msg.msg_id, raw_bytes_str, send_msg.period, ms_ago
                    ));
                    if ui.button("Remove").clicked() {
                        // Mark for removal
                        // Note: This is a bit clunky, but avoids borrowing issues
                        // We'll filter later
                        send_msg.period = -1;
                    }
                });
            }
        }

        // Add new sending
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label("MSG ID (hex):");
                ui.text_edit_singleline(&mut self.current_msg_id);
            });
            ui.horizontal(|ui| {
                ui.label("Data (hex bytes, space-separated):");
                ui.text_edit_singleline(&mut self.current_data);
            });
            ui.horizontal(|ui| {
                ui.label("Period (ms):");
                ui.text_edit_singleline(&mut self.current_period);
            });
            ui.checkbox(&mut self.current_extended, "Extended ID");
            
            if ui.button("Add message to send").clicked() {
                // Parse inputs
                if let Ok(msg_id) = u32::from_str_radix(self.current_msg_id.trim_start_matches("0x"), 16) {
                    let data_bytes_result: Result<Vec<u8>, _> = self
                        .current_data
                        .split_whitespace()
                        .map(|b_str| u8::from_str_radix(b_str, 16))
                        .collect();
                    if let Ok(msg_bytes) = data_bytes_result {
                        if let Ok(period) = self.current_period.parse::<i32>() {
                            let send_msg = SendMessage {
                                msg_id,
                                msg_id_extended: self.current_extended,
                                msg_bytes,
                                period,
                                last_sent_time: chrono::Utc::now(),
                            };
                            self.sending.push(send_msg);
                            // Clear inputs
                            self.current_msg_id.clear();
                            self.current_data.clear();
                            self.current_period.clear();
                            self.current_extended = false;
                        } else {
                            log::error!("Invalid period input");
                        }
                    } else {
                        log::error!("Invalid data bytes input");
                    }
                } else {
                    log::error!("Invalid message ID input");
                }
            }
        });

        ui.separator();

        self.sending.retain(|msg| msg.period >= 0);
        // Actually send the things
        if !self.is_paused {
            let now = chrono::Utc::now();
            for send_msg in &mut self.sending {
                let elapsed = now.signed_duration_since(send_msg.last_sent_time);
                let ms_diff = elapsed.num_milliseconds() - send_msg.period as i64;
                if ms_diff >= 0 {
                    // Send the message
                    let ui_msg = ui::ui_messages::UiMessage::SendCanMessage {
                        msg_id: send_msg.msg_id,
                        msg_id_extended: send_msg.msg_id_extended,
                        msg_bytes: send_msg.msg_bytes.clone(),
                    };
                    if let Err(e) = ui_sender.send(ui_msg) {
                        log::error!("Failed to send UI message: {}", e);
                    } else {
                        send_msg.last_sent_time = now;
                    }

                    if ms_diff > 0 {
                        log::info!(
                            "SendMessage: Sent message ID {:X} late by {} ms",
                            send_msg.msg_id,
                            ms_diff
                        );
                    }
                }
            }
        }

        egui_tiles::UiResponse::None
    }
}
