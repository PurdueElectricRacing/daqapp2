use crate::{can, connection, util, messages};

const NO_CONNECTION_SLEEP_MS: u64 = 200;
const READ_RETRY_SLEEP_MS: u64 = 2;
const BUS_LOAD_UPDATE_MS: u128 = 200;

// Returns the number of payload data bytes in the CAN frame if it was a Can2 frame
fn process_can_frame(frame: slcan::CanFrame, state: &can::state::State) -> usize {
    match frame {
        slcan::CanFrame::Can2(frame2) => {
            let decode_msg_id = util::msg_id::slcan_to_u32_with_extid_flag(&frame2.id());
            let raw_msg_id = util::msg_id::slcan_to_u32_without_extid_flag(&frame2.id());

            let data = frame2.data().unwrap_or(&[]);

            if let Some(parser) = state.parser.as_ref() {
                if let Some(decoded) = parser.decode_msg(decode_msg_id, data) {
                    let parsed_msg = messages::ParsedMessage {
                        timestamp: chrono::Local::now(),
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
                        raw_msg_id,
                        raw_msg_id,
                        data
                    );
                }
            } else {
                log::warn!(
                    "No DBC loaded. Received frame ID 0x{:X} ({}), data: {:02X?}",
                    raw_msg_id,
                    raw_msg_id,
                    data
                );
            }

            data.len()
        }
        slcan::CanFrame::CanFd(frame_fd) => {
            let msg_id_raw = util::msg_id::slcan_to_u32_without_extid_flag(&frame_fd.id());
            log::warn!(
                "Received CAN FD frame id=0x{:X} len={}",
                msg_id_raw,
                frame_fd.data().len()
            );

            let data = frame_fd.data();
            data.len()
        }
    }
}

pub fn start_can_thread(
    can_to_ui_tx: std::sync::mpsc::Sender<messages::MsgFromCan>,
    ui_to_can_rx: std::sync::mpsc::Receiver<messages::MsgFromUi>,
    selected_source: Option<connection::ConnectionSource>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut state = can::state::State::new(can_to_ui_tx, ui_to_can_rx, selected_source);

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
                        if let Some(mut old_driver) = state.driver.take() {
                            let _ = old_driver.close();
                        }
                        state.is_connected = false;
                        state
                            .can_to_ui_tx
                            .send(messages::MsgFromCan::Disconnection)
                            .expect("Failed to send disconnected message");
                        state.current_source = Some(source);
                    }
                    messages::MsgFromUi::AddSendMessage(add_send_msg) => {
                        state.add_send_message(add_send_msg);
                    }
                    messages::MsgFromUi::DeleteSendMessage { msg_id } => {
                        state.delete_send_message(msg_id);
                    }
                }
            }
            let msgs_to_send = state.send_this_tick();
            for msg in msgs_to_send {
                if let Some(ref mut active_driver) = state.driver {
                    let id = if msg.is_msg_id_extended {
                        slcan::ExtendedId::new(msg.msg_id & 0x7FFFFFFF).map(slcan::Id::Extended)
                    } else if msg.msg_id <= 0x7FF {
                        slcan::StandardId::new(msg.msg_id as u16 & 0x7FF).map(slcan::Id::Standard)
                    } else {
                        log::warn!(
                            "Invalid message ID {} for sending CAN frame (exceeds 11 bits for standard)",
                            msg.msg_id
                        );
                        None
                    };
                    if let Some(id) = id {
                        if let Some(can2_frame) = slcan::Can2Frame::new_data(id, &msg.msg_bytes) {
                            let frame = slcan::CanFrame::Can2(can2_frame);
                            match active_driver.write_frame(frame) {
                                Ok(_) => {
                                    log::info!(
                                        "Sent CAN frame with ID 0x{:X} ({}), data: {:02X?}",
                                        msg.msg_id,
                                        msg.msg_id,
                                        msg.msg_bytes
                                    );
                                    state
                                        .can_to_ui_tx
                                        .send(messages::MsgFromCan::MessageSent {
                                            msg_id: msg.msg_id,
                                            timestamp: chrono::Local::now(),
                                            amount_left: state
                                                .send_msgs
                                                .get(&msg.msg_id)
                                                .map(|info| info.amount),
                                            // If the message is removed after the send, this
                                            // will return None, which is what we want to indicate
                                            // no more sends left
                                        })
                                        .expect("Failed to send message sent confirmation");
                                }
                                Err(e) => {
                                    log::error!("Failed to send CAN frame: {:?}", e);
                                    state.is_connected = false;
                                    if let Some(ref source) = state.current_source {
                                        let error_msg = source.display_name();
                                        state
                                            .can_to_ui_tx
                                            .send(messages::MsgFromCan::ConnectionFailed(error_msg))
                                            .expect("Failed to send connection failed message");
                                    }
                                    state.driver = None;
                                }
                            }
                        } else {
                            log::error!(
                                "Cannot send CAN frame: data length {} exceeds 8 bytes",
                                msg.msg_bytes.len()
                            );
                            continue;
                        }
                    } else {
                        log::warn!("Invalid message ID {} for sending CAN frame", msg.msg_id);
                    }
                } else {
                    log::warn!("Cannot send CAN frame, no active connection");
                }
            }

            // Attempt to connect if we don't have a driver but have a source
            if state.driver.is_none() {
                if let Some(ref source) = state.current_source {
                    match can::driver::create_driver(source) {
                        Ok(new_driver) => {
                            state.driver = Some(new_driver);
                            state.is_connected = true;
                            state
                                .can_to_ui_tx
                                .send(messages::MsgFromCan::ConnectionSuccessful)
                                .expect("Failed to send connection successful message");
                            log::info!("Connected to {:?}", source);
                        }
                        Err(e) => {
                            log::error!("Failed to create driver for {:?}: {:?}", source, e);
                            let error_msg = source.display_name();
                            state
                                .can_to_ui_tx
                                .send(messages::MsgFromCan::ConnectionFailed(error_msg))
                                .expect("Failed to send connection failed message");
                            std::thread::sleep(std::time::Duration::from_millis(
                                NO_CONNECTION_SLEEP_MS,
                            ));
                            continue;
                        }
                    }
                } else {
                    // No source configured, just sleep
                    std::thread::sleep(std::time::Duration::from_millis(NO_CONNECTION_SLEEP_MS));
                    continue;
                }
            }

            // Try to read a frame from the driver
            let Some(ref mut active_driver) = state.driver else {
                std::thread::sleep(std::time::Duration::from_millis(NO_CONNECTION_SLEEP_MS));
                continue;
            };

            match active_driver.read_frame() {
                Ok(frame) => {
                    let data_bytes = process_can_frame(frame, &state);
                    state.bus_load_tracker.record_frame(data_bytes);

                    // Send bus load updates periodically
                    if state.last_bus_load_update.elapsed().as_millis() >= BUS_LOAD_UPDATE_MS {
                        state.bus_load_tracker.cleanup();
                        let load_1s = state.bus_load_tracker.get_load(1);
                        let load_5s = state.bus_load_tracker.get_load(5);
                        let load_10s = state.bus_load_tracker.get_load(10);
                        let load_30s = state.bus_load_tracker.get_load(30);

                        state
                            .can_to_ui_tx
                            .send(messages::MsgFromCan::BusLoad {
                                load_1s,
                                load_5s,
                                load_10s,
                                load_30s,
                            })
                            .expect("Failed to send bus load message");

                        state.last_bus_load_update = std::time::Instant::now();
                    }
                }
                Err(can::driver::DriverError::ReadError(error_type)) => {
                    match error_type {
                        can::driver::DriverReadError::Timeout => {
                            // Normal timeout, just retry
                            std::thread::sleep(std::time::Duration::from_millis(
                                READ_RETRY_SLEEP_MS,
                            ));
                        }
                        other => {
                            // Actual error, disconnect
                            log::error!("Driver read error: {:?}", other);
                            state.is_connected = false;
                            if let Some(ref source) = state.current_source {
                                let error_msg = source.display_name();
                                state
                                    .can_to_ui_tx
                                    .send(messages::MsgFromCan::ConnectionFailed(error_msg))
                                    .expect("Failed to send connection failed message");
                            }
                            state.driver = None;
                        }
                    }
                }
                Err(e) => {
                    log::error!("Unexpected driver error: {:?}", e);
                    state.is_connected = false;
                    state.driver = None;
                }
            }
        }
        unreachable!("CAN thread should never exit on its own");
    })
}
