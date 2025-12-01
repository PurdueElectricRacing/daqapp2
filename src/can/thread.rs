use crate::{can, ui};
use chrono::Local;
use serialport::ClearBuffer;
use slcan::sync::CanSocket;
use slcan::{CanFrame, NominalBitRate, OperatingMode, ReadError};
use std::{io, thread, time::Duration};

pub fn start_can_thread(
    can_sender: std::sync::mpsc::Sender<can::can_messages::CanMessage>,
    ui_receiver: std::sync::mpsc::Receiver<ui::ui_messages::UiMessage>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut state = can::state::State::new(can_sender, ui_receiver);
        let serial_path = "/dev/ttyACM0";
        let baud_rate = 115_200u32;

        // --- Open serial port ---
        let port = match serialport::new(serial_path, baud_rate)
            .timeout(Duration::from_millis(10))
            .open()
        {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to open serialport {e}");
                return;
            }
        };

        let _ = port.clear(ClearBuffer::All);

        // --- Wrap into SLCAN FD socket ---
        let mut can = CanSocket::new(port.try_clone().expect("clone serialport failed"));

        // Reset, set mode, open, etc.
        if let Err(e) = can.set_operating_mode(OperatingMode::Normal) {
            log::error!("Failed to set operating mode: {e}");
            return;
        }

        if let Err(e) = can.open(NominalBitRate::Rate500Kbit) {
            log::error!("Failed to open CAN: {e}");
            return;
        }

        state.is_connected = true;

        // --- Main loop ---
        loop {
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
                }
            }

            // Try to read a frame
            match can.read() {
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
                    thread::sleep(Duration::from_millis(2));
                }
                Err(e) => {
                    log::error!("Read error: {e}");
                    break;
                }
            }
        }
        let _ = can.close();
        log::info!("Exiting CAN thread");
    })
}
