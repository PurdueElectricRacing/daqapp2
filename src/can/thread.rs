use crate::{can, ui};
use chrono::Local;
use slcan::CanFrame;
use std::{io, sync::mpsc, thread, time::Duration};

const READ_RETRY_SLEEP_MS: u64 = 2;
const IDLE_SLEEP_MS: u64 = 50;

pub fn start_can_thread(
    can_sender: mpsc::Sender<can::can_messages::CanMessage>,
    ui_receiver: mpsc::Receiver<ui::ui_messages::UiMessage>,
    selected_serial: Option<String>,
    dbc_path: Option<std::path::PathBuf>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut state = can::state::State::new(can_sender, ui_receiver);
        let mut driver: Option<Box<dyn can::CanDriver>> = None;

        if let Some(path) = dbc_path {
            match can_decode::Parser::from_dbc_file(&path) {
                Ok(parser) => {
                    state.parser = Some(parser);
                    log::info!("Loaded DBC from settings: {:?}", path);
                }
                Err(e) => log::error!("Failed to load DBC from settings {:?}: {e}", path),
            }
        }

        // Handle initial serial if provided
        if let Some(path) = selected_serial {
            log::info!("Attempting initial connection to {}", path);
            let d = can::serial::SerialDriver::new(path.clone(), 115_200);
            driver = Some(Box::new(d));
        }

        loop {
            // 1. Process UI messages
            while let Ok(msg) = state.ui_receiver.try_recv() {
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
                        log::info!("Connecting to new source...");
                        driver = None; // Drop old driver
                        match source {
                            ui::ui_messages::ConnectionSource::Serial(path) => {
                                let d = can::serial::SerialDriver::new(path, 115_200);
                                driver = Some(Box::new(d));
                            }
                            ui::ui_messages::ConnectionSource::Udp(port) => {
                                match can::udp::UdpDriver::new(port) {
                                    Ok(d) => driver = Some(Box::new(d)),
                                    Err(e) => {
                                        log::error!("Failed to open UDP: {e}");
                                        let _ = state.can_sender.send(can::can_messages::CanMessage::ConnectionFailed("UDP Error".into()));
                                    }
                                }
                            }
                        }
                    }
                    ui::ui_messages::UiMessage::Disconnect => {
                        log::info!("Disconnecting hardware...");
                        driver = None;
                        state.is_connected = false;
                    }
                }
            }

            // 2. Read from driver if it exists
            if let Some(ref mut d) = driver {
                match d.read_frame() {
                    Ok(frame) => {
                        if !state.is_connected {
                            state.is_connected = true;
                            let _ = state.can_sender.send(can::can_messages::CanMessage::ConnectionSuccessful);
                        }
                        process_frame(&mut state, frame);
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut => {
                        thread::sleep(Duration::from_millis(READ_RETRY_SLEEP_MS));
                    }
                    Err(e) => {
                        log::error!("Driver read error: {e}");
                        state.is_connected = false;
                        let _ = state.can_sender.send(can::can_messages::CanMessage::ConnectionFailed(e.to_string()));
                        driver = None;
                    }
                }
            } else {
                // No driver, sleep longer
                thread::sleep(Duration::from_millis(IDLE_SLEEP_MS));
            }
        }
    })
}

fn process_frame(state: &mut can::state::State, frame: CanFrame) {
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
                }
            } else {
                // log::warn!("No DBC loaded for ID 0x{:X}", id);
            }
        }
        CanFrame::CanFd(frame_fd) => {
            log::info!("Received CAN FD frame: {:?}", frame_fd);
        }
    }
}
