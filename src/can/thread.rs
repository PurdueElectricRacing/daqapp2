use crate::{can, connection, messages};
use chrono::Local;
use slcan::CanFrame;
use std::{thread, time::Duration};

const NO_CONNECTION_SLEEP_MS: u64 = 200;
const READ_RETRY_SLEEP_MS: u64 = 2;

fn process_can_frame(frame: CanFrame, state: &can::state::State) {
    match frame {
        CanFrame::Can2(frame2) => {
            let id = match frame2.id() {
                slcan::Id::Standard(sid) => sid.as_raw() as u32,
                slcan::Id::Extended(eid) => eid.as_raw(),
            };

            let data = frame2.data().unwrap_or(&[]);

            if let Some(parser) = state.parser.as_ref() {
                if let Some(decoded) = parser.decode_msg(id, data) {
                    let parsed_msg = messages::ParsedMessage {
                        timestamp: Local::now(),
                        raw_bytes: data.to_vec(),
                        decoded,
                    };
                    state
                        .can_to_ui_tx
                        .send(messages::MsgFromCan::ParsedMessage(parsed_msg))
                        .expect("Failed to send parsed CAN message");
                } else {
                    log::error!(
                        "Failed to parse: frame ID 0x{:X} ({}), data: {:02X?}",
                        id,
                        id,
                        data
                    );
                }
            } else {
                log::warn!(
                    "No DBC loaded. Received frame ID 0x{:X} ({}), data: {:02X?}",
                    id,
                    id,
                    data
                );
            }
        }
        CanFrame::CanFd(frame_fd) => {
            let id = match frame_fd.id() {
                slcan::Id::Standard(sid) => sid.as_raw() as u32,
                slcan::Id::Extended(eid) => eid.as_raw(),
            };
            log::warn!(
                "Received CAN FD frame id=0x{:X} len={}",
                id,
                frame_fd.data().len()
            );
        }
    }
}

pub fn start_can_thread(
    can_to_ui_tx: std::sync::mpsc::Sender<messages::MsgFromCan>,
    ui_to_can_rx: std::sync::mpsc::Receiver<messages::MsgFromUi>,
    selected_source: Option<connection::ConnectionSource>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut state = can::state::State::new(can_to_ui_tx, ui_to_can_rx);
        let mut driver: Option<Box<dyn can::driver::Driver>> = None;
        let mut current_source: Option<connection::ConnectionSource> = selected_source;

        // MAIN LOOP
        loop {
            // Process UI messages first (DBC load, new message to send, etc.)
            while let Ok(msg) = state.ui_to_can_rx.try_recv() {
                match msg {
                    messages::MsgFromUi::DbcSelected(path) => {
                        match can_decode::Parser::from_dbc_file(&path) {
                            Ok(parser) => {
                                state.parser = Some(parser);
                                log::info!("Loaded DBC from {:?}", path);
                            }
                            Err(e) => log::error!("Failed to load DBC {:?}: {e}", path),
                        }
                    }
                    messages::MsgFromUi::Connect(source) => {
                        // Close existing connection if any
                        if let Some(mut old_driver) = driver.take() {
                            let _ = old_driver.close();
                        }
                        state.is_connected = false;
                        state
                            .can_to_ui_tx
                            .send(messages::MsgFromCan::Disconnection)
                            .expect("Failed to send disconnected message");
                        current_source = Some(source);
                    }
                    messages::MsgFromUi::AddSendMessage(add_send_msg) => {
                        state.add_send_message(add_send_msg);
                    }
                    messages::MsgFromUi::DeleteSendMessage { msg_id } => {
                        state.delete_send_message(msg_id);
                    }
                }
            }
            // Process send thread messages (requests to send CAN frames)
            // for msg in state.send_to_can_rx.try_iter() {
            //     match msg {
            //         send::messages::FromSendThreadToCan::Send {
            //             msg_id,
            //             is_msg_id_extended,
            //             msg_bytes,
            //         } => {
            //             if let Some(ref mut active_driver) = driver {
            //                 let id = if is_msg_id_extended {
            //                     slcan::Id::Extended(slcan::ExtendedId::new(msg_id).unwrap())
            //                 } else {
            //                     slcan::Id::Standard(slcan::StandardId::new(msg_id as u16).unwrap())
            //                 };
            //                 if let Some(can2_frame) = slcan::Can2Frame::new_data(id, &msg_bytes) {
            //                     let frame = CanFrame::Can2(can2_frame);
            //                     match active_driver.write_frame(frame) {
            //                         Ok(_) => {
            //                             log::info!(
            //                                 "Sent CAN frame with ID 0x{:X} ({}), data: {:02X?}",
            //                                 msg_id,
            //                                 msg_id,
            //                                 msg_bytes
            //                             );
            //                             // TODO
            //                             // state.send_to_ui_tx
            //                             //     .send(send::messages::FromSendThreadToUi::MessageSent {
            //                             //         msg_id,
            //                             //         timestamp: Local::now(),
            //                             //     })
            //                             //     .expect("Failed to send message sent confirmation");
            //                         }
            //                         Err(e) => {
            //                             log::error!("Failed to send CAN frame: {:?}", e);
            //                             state.is_connected = false;
            //                             let error_msg = if let Some(ref source) = current_source {
            //                                 match source {
            //                                     connection::ConnectionSource::Serial(path) => {
            //                                         path.clone()
            //                                     }
            //                                     connection::ConnectionSource::Udp(port) => {
            //                                         format!("UDP:{}", port)
            //                                     }
            //                                 }
            //                             } else {
            //                                 "Unknown source".to_string()
            //                             };
            //                             state
            //                                 .can_to_ui_tx
            //                                 .send(can::can_messages::CanMessage::ConnectionFailed(
            //                                     error_msg,
            //                                 ))
            //                                 .expect("Failed to send connection failed message");
            //                             driver = None;
            //                         }
            //                     }
            //                 } else {
            //                     log::error!(
            //                         "Cannot send CAN frame: data length {} exceeds 8 bytes",
            //                         msg_bytes.len()
            //                     );
            //                     continue;
            //                 }
            //             } else {
            //                 log::warn!("Cannot send CAN frame, no active connection");
            //             }
            //         }
            //     }
            // }

            // Attempt to connect if we don't have a driver but have a source
            if driver.is_none() {
                if let Some(ref source) = current_source {
                    match can::driver::create_driver(source) {
                        Ok(new_driver) => {
                            driver = Some(new_driver);
                            state.is_connected = true;
                            state
                                .can_to_ui_tx
                                .send(messages::MsgFromCan::ConnectionSuccessful)
                                .expect("Failed to send connection successful message");
                            log::info!("Connected to {:?}", source);
                        }
                        Err(e) => {
                            log::error!("Failed to create driver for {:?}: {:?}", source, e);
                            let error_msg = match source {
                                connection::ConnectionSource::Serial(path) => path.clone(),
                                connection::ConnectionSource::Udp(port) => format!("UDP:{}", port),
                            };
                            state
                                .can_to_ui_tx
                                .send(messages::MsgFromCan::ConnectionFailed(error_msg))
                                .expect("Failed to send connection failed message");
                            thread::sleep(Duration::from_millis(NO_CONNECTION_SLEEP_MS));
                            continue;
                        }
                    }
                } else {
                    // No source configured, just sleep
                    thread::sleep(Duration::from_millis(NO_CONNECTION_SLEEP_MS));
                    continue;
                }
            }

            // Try to read a frame from the driver
            let Some(ref mut active_driver) = driver else {
                thread::sleep(Duration::from_millis(NO_CONNECTION_SLEEP_MS));
                continue;
            };

            match active_driver.read_frame() {
                Ok(frame) => {
                    process_can_frame(frame, &state);
                }
                Err(can::driver::DriverError::ReadError(error_type)) => {
                    match error_type {
                        can::driver::DriverReadError::Timeout => {
                            // Normal timeout, just retry
                            thread::sleep(Duration::from_millis(READ_RETRY_SLEEP_MS));
                        }
                        other => {
                            // Actual error, disconnect
                            log::error!("Driver read error: {:?}", other);
                            state.is_connected = false;
                            if let Some(ref source) = current_source {
                                let error_msg = match source {
                                    connection::ConnectionSource::Serial(path) => path.clone(),
                                    connection::ConnectionSource::Udp(port) => {
                                        format!("UDP:{}", port)
                                    }
                                };
                                state
                                    .can_to_ui_tx
                                    .send(messages::MsgFromCan::ConnectionFailed(error_msg))
                                    .expect("Failed to send connection failed message");
                            }
                            driver = None;
                        }
                    }
                }
                Err(e) => {
                    log::error!("Unexpected driver error: {:?}", e);
                    state.is_connected = false;
                    driver = None;
                }
            }
        }
        unreachable!("CAN thread should never exit on its own");
    })
}
