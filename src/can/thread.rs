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
        let serial_path = "/dev/pts/2";
        let baud_rate = 115_200u32;

        // --- Open serial port ---
        let mut port = match serialport::new(serial_path, baud_rate)
            .timeout(Duration::from_millis(10))
            .open()
        {
            Ok(p) => p,
            Err(e) => {
                return;
            }
        };

        let _ = port.clear(ClearBuffer::All);

        // --- Wrap into SLCAN FD socket ---
        let mut can = CanSocket::new(port.try_clone().expect("clone serialport failed"));

        // Reset, set mode, open, etc.
        if let Err(e) = can.set_operating_mode(OperatingMode::Normal) {
            eprintln!("[can-thread] Failed to set operating mode: {e}");
            return;
        }

        if let Err(e) = can.open(NominalBitRate::Rate500Kbit) {
            eprintln!("[can-thread] Failed to open CAN: {e}");
            return;
        }

        state.is_connected = true;

        // --- Main loop ---
        let mut raw_buf = [0u8; 128];
        loop {
            match port.read(&mut raw_buf) {
                Ok(n) if n > 0 => {
                    let data = &raw_buf[..n];
                    let hex_str = data
                        .iter()
                        .map(|b| format!("{:02X} ", b))
                        .collect::<String>();
                    let ascii_str = data
                        .iter()
                        .map(|b| {
                            if b.is_ascii_graphic() {
                                *b as char
                            } else {
                                '.'
                            }
                        })
                        .collect::<String>();
                    println!(
                        "[can-thread][RAW] {} bytes: [{}]  ASCII: \"{}\"",
                        n,
                        hex_str.trim_end(),
                        ascii_str
                    );
                }
                Ok(_) => {}
                Err(ref e)
                    if e.kind() == io::ErrorKind::WouldBlock
                        || e.kind() == io::ErrorKind::TimedOut =>
                {
                    // no data ready yet
                }
                Err(e) => {
                    eprintln!("[can-thread] Serial read error: {e}");
                    break;
                }
            }

            // Process UI messages first (DBC load, etc.)
            for msg in state.ui_receiver.try_iter() {
                match msg {
                    ui::ui_messages::UiMessage::DbcSelected(path) => {
                        match can_decode::Parser::from_dbc_file(&path) {
                            Ok(parser) => {
                                state.parser = Some(parser);
                                println!("[can-thread] Loaded DBC from {:?}", path);
                            }
                            Err(e) => eprintln!("[can-thread] Failed to load DBC {:?}: {e}", path),
                        }
                    }
                }
            }

            // Try to read a frame
            match can.read() {
                Ok(frame) => match frame {
                    CanFrame::Can2(frame2) => {
                        println!("[can-thread] Received frame: {:?}", frame2);
                        if let Some(parser) = state.parser.as_ref() {
                            let id = match frame2.id() {
                                slcan::Id::Standard(sid) => sid.as_raw() as u32,
                                slcan::Id::Extended(eid) => eid.as_raw(),
                            };
                            let data = frame2.data();

                            if let Some(decoded) = parser.decode_msg(id, data.expect("")) {
                                let parsed_msg = can::message::ParsedMessage {
                                    timestamp: Local::now(),
                                    decoded,
                                };
                                let _ = state
                                    .can_sender
                                    .send(can::can_messages::CanMessage::ParsedMessage(parsed_msg));
                            }
                        }
                    }
                    CanFrame::CanFd(frame_fd) => {
                        // Optional: Handle FD frames differently or log
                        println!("[can-thread] Received frame: {:?}", frame_fd);
                        let id = match frame_fd.id() {
                            slcan::Id::Standard(sid) => sid.as_raw() as u32,
                            slcan::Id::Extended(eid) => eid.as_raw(),
                        };
                        eprintln!(
                            "[can-thread] Received CAN FD frame id=0x{:X} len={}",
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
                    eprintln!("[can-thread] read error: {e}");
                    break;
                }
            }
        }
        let _ = can.close();
        println!("[can-thread] Exiting CAN thread");
    })
}
