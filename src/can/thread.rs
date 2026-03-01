use crate::{can, connection, ui};
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
                    let parsed_msg = can::message::ParsedMessage {
                        timestamp: Local::now(),
                        raw_bytes: data.to_vec(),
                        decoded,
                    };
                    let _ = state
                        .can_sender
                        .send(can::can_messages::CanMessage::ParsedMessage(parsed_msg));
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
    can_sender: std::sync::mpsc::Sender<can::can_messages::CanMessage>,
    ui_receiver: std::sync::mpsc::Receiver<ui::ui_messages::UiMessage>,
    selected_source: Option<connection::ConnectionSource>,
    dbc_path: Option<std::path::PathBuf>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut state = can::state::State::new(can_sender, ui_receiver);
        let mut driver: Option<Box<dyn can::driver::Driver>> = None;
        let mut pending_connection_error: Option<String> = None;
        let mut current_source: Option<connection::ConnectionSource> = selected_source;

        if let Some(path) = dbc_path {
            match can_decode::Parser::from_dbc_file(&path) {
                Ok(parser) => {
                    state.parser = Some(parser);
                    log::info!("Loaded DBC from settings: {:?}", path);
                }
                Err(e) => log::error!("Failed to load DBC from settings {:?}: {e}", path),
            }
        }

        // MAIN LOOP
        loop {
            if let Some(error_msg) = pending_connection_error.take() {
                let _ = state
                    .can_sender
                    .send(can::can_messages::CanMessage::ConnectionFailed(error_msg));
            }
            // Process UI messages first (DBC load, etc.)
            for msg in state.ui_receiver.try_iter() {
                match msg {
                    ui::ui_messages::UiMessage::DbcSelected(path) => {
                        match can_decode::Parser::from_dbc_file(&path) {
                            Ok(parser) => {
                                state.parser = Some(parser);
                                log::info!("Loaded DBC from {:?}", path);
                            }
                            Err(e) => log::error!("Failed to load DBC {:?}: {e}", path),
                        }
                    }
                    ui::ui_messages::UiMessage::Connect(source) => {
                        // Close existing connection if any
                        if let Some(mut old_driver) = driver.take() {
                            let _ = old_driver.close();
                        }
                        state.is_connected = false;
                        current_source = Some(source);
                    }
                }
            }

            // Attempt to connect if we don't have a driver but have a source
            if driver.is_none() {
                if let Some(ref source) = current_source {
                    match can::driver::create_driver(source) {
                        Ok(new_driver) => {
                            driver = Some(new_driver);
                            state.is_connected = true;
                            let _ = state
                                .can_sender
                                .send(can::can_messages::CanMessage::ConnectionSuccessful);
                            log::info!("Connected to {:?}", source);
                        }
                        Err(e) => {
                            log::error!("Failed to create driver for {:?}: {:?}", source, e);
                            let error_msg = match source {
                                connection::ConnectionSource::Serial(path) => path.clone(),
                                connection::ConnectionSource::Udp(port) => format!("UDP:{}", port),
                            };
                            pending_connection_error = Some(error_msg);
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
                Err(can::driver::DriverError::ReadError(msg)) => {
                    if msg == "Timeout" {
                        // Normal timeout, just retry
                        log::warn!("Driver read timeout, retrying...");
                        thread::sleep(Duration::from_millis(READ_RETRY_SLEEP_MS));
                    } else {
                        // Actual error, disconnect
                        log::error!("Driver read error: {}", msg);
                        state.is_connected = false;
                        if let Some(ref source) = current_source {
                            let error_msg = match source {
                                connection::ConnectionSource::Serial(path) => path.clone(),
                                connection::ConnectionSource::Udp(port) => format!("UDP:{}", port),
                            };
                            pending_connection_error = Some(error_msg);
                        }
                        driver = None;
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
