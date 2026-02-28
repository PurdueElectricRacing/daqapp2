use crate::{can, ui};
use chrono::Local;
use serialport::ClearBuffer;
use serialport::SerialPort;
use slcan::sync::CanSocket;
use slcan::{CanFrame, NominalBitRate, OperatingMode, ReadError};
use std::{io, thread, time::Duration};

const BAUD_RATE: u32 = 115_200;
const NO_PORT_SLEEP_MS: u64 = 200;
const READ_RETRY_SLEEP_MS: u64 = 2;

pub fn start_can_thread(
    can_sender: std::sync::mpsc::Sender<can::can_messages::CanMessage>,
    ui_receiver: std::sync::mpsc::Receiver<ui::ui_messages::UiMessage>,
    selected_serial: Option<String>,
    dbc_path: Option<std::path::PathBuf>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut state = can::state::State::new(can_sender, ui_receiver);
        let mut can: Option<CanSocket<Box<dyn SerialPort>>> = None;
        let mut pending_connection_error: Option<String> = None;
        let mut serial_path: Option<String> = selected_serial;

        if let Some(path) = dbc_path {
            match can_decode::Parser::from_dbc_file(&path) {
                Ok(parser) => {
                    state.parser = Some(parser);
                    log::info!("Loaded DBC from settings: {:?}", path);
                }
                Err(e) => log::error!("Failed to load DBC from settings {:?}: {e}", path),
            }
        }
        // --- Main loop ---
        loop {
            if let Some(path) = pending_connection_error.take() {
                let _ = state
                    .can_sender
                    .send(can::can_messages::CanMessage::ConnectionFailed(path));
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
                    ui::ui_messages::UiMessage::SerialSelected(path) => {
                        // Close existing connection if any
                        if let Some(mut old) = can.take() {
                            let _ = old.close();
                        }

                        state.is_connected = false;
                        serial_path = Some(path);
                    }
                }
            }

            if can.is_none()
                && let Some(ref path) = serial_path
            {
                let port = match serialport::new(path, BAUD_RATE)
                    .timeout(Duration::from_millis(10))
                    .open()
                {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("Failed to open serialport {}: {e}", path);
                        pending_connection_error = Some(path.clone());
                        thread::sleep(Duration::from_millis(NO_PORT_SLEEP_MS));
                        continue;
                    }
                };
                let _ = port.clear(ClearBuffer::All);
                let mut socket = CanSocket::new(port.try_clone().expect("clone serialport failed"));
                if let Err(e) = socket.set_operating_mode(OperatingMode::Normal) {
                    log::error!("Failed to set operating mode: {e}");
                    state.is_connected = false;
                    pending_connection_error = Some(path.clone());
                    thread::sleep(Duration::from_millis(NO_PORT_SLEEP_MS));
                    continue;
                }
                if let Err(e) = socket.open(NominalBitRate::Rate500Kbit) {
                    log::error!("Failed to open CAN: {e}");
                    state.is_connected = false;
                    pending_connection_error = Some(path.clone());
                    thread::sleep(Duration::from_millis(NO_PORT_SLEEP_MS));
                    continue;
                }
                state.is_connected = true;
                let _ = state
                    .can_sender
                    .send(can::can_messages::CanMessage::ConnectionSuccessful);
                can = Some(socket);
            }

            // Try to read a frame
            let Some(ref mut can_socket) = can else {
                thread::sleep(Duration::from_millis(NO_PORT_SLEEP_MS));
                continue;
            };
            match can_socket.read() {
                Ok(frame) => match frame {
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
                            continue;
                        };
                    }
                    CanFrame::CanFd(frame_fd) => {
                        // Optional: Handle FD frames differently or log
                        log::info!("Received frame: {:?}", frame_fd);
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
                },
                Err(ReadError::Io(e))
                    if e.kind() == io::ErrorKind::WouldBlock
                        || e.kind() == io::ErrorKind::TimedOut =>
                {
                    thread::sleep(Duration::from_millis(READ_RETRY_SLEEP_MS));
                }
                Err(e) => {
                    log::error!("Read error: {e}");
                    state.is_connected = false;
                    pending_connection_error = serial_path.clone();
                    can = None;
                    continue;
                }
            }
        }
        unreachable!("CAN thread should never exit on its own");
    })
}
