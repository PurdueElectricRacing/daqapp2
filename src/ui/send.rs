use crate::ui;
use chrono::Timelike;
use chrono::Utc;
use eframe::egui;
use std::collections::HashMap;

struct SendMessage {
    msg_id: u32,
    msg_id_extended: bool,
    msg_bytes: Vec<u8>,
    period: i32, // milliseconds (negative = means ignore)
    last_sent_time: chrono::DateTime<chrono::Utc>,
    signals: HashMap<String, f64>,
}

pub struct Send {
    pub title: String,
    sending: Vec<SendMessage>,
    is_paused: bool,
    // current_msg_id: String,
    // current_data: String,
    current_period: String,
    // current_extended: bool,
    dbc_path: Option<std::path::PathBuf>,
    parser: Option<can_decode::Parser>,
    msg_search: String,
    selected_msg: Option<can_dbc::Message>,
    signals_input: Vec<String>, // parallel to selected_msg signals
}

impl Send {
    pub fn new(instance_num: usize) -> Self {
        let title = format!("Send #{}", instance_num);
        Self {
            title,
            sending: Vec::new(),
            is_paused: false,
            // current_msg_id: String::new(),
            // current_data: String::new(),
            current_period: String::new(),
            // current_extended: false,
            dbc_path: None,
            parser: None,
            msg_search: String::new(),
            selected_msg: None,
            signals_input: Vec::new(),
        }
    }

    fn select_dbc(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            // .add_filter("DBC Files", &["dbc"])
            .pick_file()
        {
            self.dbc_path = Some(path);
            self.parser = match can_decode::Parser::from_dbc_file(self.dbc_path.as_ref().unwrap()) {
                Ok(p) => {
                    log::info!("Loaded DBC from {:?}", self.dbc_path.as_ref().unwrap());
                    Some(p)
                }
                Err(e) => {
                    log::error!(
                        "Failed to load DBC {:?}: {}",
                        self.dbc_path.as_ref().unwrap(),
                        e
                    );
                    None
                }
            };
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

        // DBC file selection
        ui.horizontal(|ui| {
            if ui.button("📁 Select DBC").clicked() {
                self.select_dbc();
            }
            if let Some(path) = &self.dbc_path {
                ui.label(format!("DBC: {}", path.display()));
            } else {
                ui.label("DBC: None selected");
            }
        });
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
                        // We'll filter later, mark for removal to avoid borrow issues
                        send_msg.period = -1;
                    }
                });
            }
        }

        // Add new sending
        if let Some(parser) = &self.parser {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("search MSG name:");
                    ui.text_edit_singleline(&mut self.msg_search);
                });

                let mut filtered_msgs = Vec::new();
                for msg in parser.msg_defs() {
                    if msg.name.to_lowercase().contains(&self.msg_search.to_lowercase()) {
                        filtered_msgs.push(msg.clone());
                    }
                }

                if filtered_msgs.len() == 1 {
                    self.selected_msg = Some(filtered_msgs[0].clone());
                    ui.label(format!("Selected message: {}", filtered_msgs[0].name));
                    if self.signals_input.len() != filtered_msgs[0].signals.len() {
                        self.signals_input = vec!["0.0".to_string(); filtered_msgs[0].signals.len()];
                    }
                    for (i, signal) in filtered_msgs[0].signals.iter().enumerate()
                    {
                        ui.horizontal(|ui| {
                            ui.label(format!("Signal '{}':", signal.name));
                            ui.text_edit_singleline(&mut self.signals_input[i]);
                        });
                    }

                    ui.horizontal(|ui| {
                        ui.label("Period (ms):");
                        ui.text_edit_singleline(&mut self.current_period);
                    });

                    if ui.button("Add message to send").clicked() {
                        // Parse period
                        let period = match self.current_period.parse::<i32>() {
                            Ok(p) => p,
                            Err(e) => {
                                log::error!("Invalid period input: {}", e);
                                return;
                            }
                        };

                        // Prepare signal values
                        let mut signal_values = HashMap::new();
                        for (i, signal) in filtered_msgs[0].signals.iter().enumerate() {
                            if let Ok(val) = self.signals_input[i].parse::<f64>() {
                                signal_values.insert(signal.name.clone(), val);
                            } else {
                                log::error!(
                                    "Invalid value for signal '{}': {}",
                                    signal.name,
                                    self.signals_input[i]
                                );
                                return;
                            }
                        }

                        // Encode message
                        let id_raw = match filtered_msgs[0].id {
                                can_dbc::MessageId::Standard(id) => id as u32,
                                can_dbc::MessageId::Extended(id) => id,
                            };
                            let is_extended = matches!(filtered_msgs[0].id, can_dbc::MessageId::Extended(_));
                        match parser.encode_msg(
                            id_raw,
                            &signal_values,
                        ) {
                            Some(msg_bytes) => {
                                let send_msg = SendMessage {
                                    msg_id: id_raw,
                                    msg_id_extended: is_extended,
                                    msg_bytes,
                                    period,
                                    last_sent_time: chrono::DateTime::<Utc>::UNIX_EPOCH,
                                    signals: signal_values,
                                };
                                self.sending.push(send_msg);
                                log::info!(
                                    "Added message '{}' (ID {:X}) to sending list",
                                    filtered_msgs[0].name,
                                    id_raw
                                );
                            }
                            None => {
                                log::error!(
                                    "Failed to encode message '{}'",
                                    filtered_msgs[0].name,
                                );
                            }
                        }
                    }
                } else {
                    ui.label(format!("{} messages found. Need 1.", filtered_msgs.len()));
                }
            });
        } else {
            ui.label("Load a DBC file to select messages to send.");
        }

        // ui.vertical(|ui| {
        //     ui.horizontal(|ui| {
        //         ui.label("MSG ID (hex):");
        //         ui.text_edit_singleline(&mut self.current_msg_id);
        //     });
        //     ui.horizontal(|ui| {
        //         ui.label("Data (hex bytes, space-separated):");
        //         ui.text_edit_singleline(&mut self.current_data);
        //     });
        //     ui.horizontal(|ui| {
        //         ui.label("Period (ms):");
        //         ui.text_edit_singleline(&mut self.current_period);
        //     });
        //     ui.checkbox(&mut self.current_extended, "Extended ID");

        //     if ui.button("Add message to send").clicked() {
        //         // Parse inputs
        //         if let Ok(msg_id) = u32::from_str_radix(self.current_msg_id.trim_start_matches("0x"), 16) {
        //             let data_bytes_result: Result<Vec<u8>, _> = self
        //                 .current_data
        //                 .split_whitespace()
        //                 .map(|b_str| u8::from_str_radix(b_str, 16))
        //                 .collect();
        //             if let Ok(msg_bytes) = data_bytes_result {
        //                 if let Ok(period) = self.current_period.parse::<i32>() {
        //                     let send_msg = SendMessage {
        //                         msg_id,
        //                         msg_id_extended: self.current_extended,
        //                         msg_bytes,
        //                         period,
        //                         last_sent_time: chrono::Utc::now(),
        //                     };
        //                     self.sending.push(send_msg);
        //                     // Clear inputs
        //                     self.current_msg_id.clear();
        //                     self.current_data.clear();
        //                     self.current_period.clear();
        //                     self.current_extended = false;
        //                 } else {
        //                     log::error!("Invalid period input");
        //                 }
        //             } else {
        //                 log::error!("Invalid data bytes input");
        //             }
        //         } else {
        //             log::error!("Invalid message ID input");
        //         }
        //     }
        // });

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
