use crate::can::{
    ConnectionSource,
    can_messages::{CanMessage, WorkerCommand},
    message::ParsedMessage,
};
use chrono::Local;
use slcan::CanFrame;
use std::{io, path::PathBuf, sync::mpsc, thread, time::Duration};

const READ_RETRY_SLEEP_MS: u64 = 2;
const RECONNECT_DELAY_MS: u64 = 1000;

pub fn spawn_worker(
    can_sender: mpsc::Sender<CanMessage>,
    command_receiver: mpsc::Receiver<WorkerCommand>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut driver: Option<Box<dyn crate::can::CanDriver>> = None;
        let mut source_info: Option<(ConnectionSource, String)> = None;
        let mut parser: Option<can_decode::Parser> = None;
        let mut first_success = false;

        log::info!("CAN persistent worker started");

        loop {
            // 1. Handle commands
            let cmd = if driver.is_none() && source_info.is_none() {
                // Fully idle: block until we get a command
                match command_receiver.recv() {
                    Ok(c) => Some(c),
                    Err(_) => {
                        log::info!("CAN worker command channel disconnected, shutting down");
                        return;
                    }
                }
            } else {
                // Active or retrying: non-blocking check
                command_receiver.try_recv().ok()
            };

            if let Some(c) = cmd {
                match c {
                    WorkerCommand::Shutdown => {
                        log::info!("CAN worker received Shutdown");
                        return;
                    }
                    WorkerCommand::Disconnect => {
                        log::info!("CAN worker Disconnect");
                        driver = None;
                        source_info = None;
                    }
                    WorkerCommand::UpdateDbc(path) => {
                        parser = load_parser(path);
                    }
                    WorkerCommand::Connect { source, dbc_path } => {
                        let name = match &source {
                            ConnectionSource::Serial(p) => p.clone(),
                            ConnectionSource::Udp(p) => format!("UDP:{}", p),
                        };
                        log::info!("CAN worker connecting to {}", name);
                        source_info = Some((source, name));
                        parser = load_parser(dbc_path);
                        first_success = true;
                        driver = None; // Force new connection attempt
                    }
                }
            }

            // 2. If we have a source but no active driver, try to connect/reconnect
            if driver.is_none() {
                if let Some((source, name)) = &source_info {
                    match source.create_driver() {
                        Ok(d) => {
                            driver = Some(d);
                        }
                        Err(e) => {
                            let _ = can_sender.send(CanMessage::ConnectionFailed {
                                source: name.clone(),
                                error: e.to_string(),
                            });
                            thread::sleep(Duration::from_millis(RECONNECT_DELAY_MS));
                            continue; // Check for commands again
                        }
                    }
                }
            }

            // 3. If we have a driver, try to read from it
            if let Some(ref mut d) = driver {
                match d.read_frame() {
                    Ok(frame) => {
                        if first_success {
                            let _ = can_sender.send(CanMessage::ConnectionSuccessful);
                            first_success = false;
                        }
                        process_frame(&can_sender, &mut parser, frame);
                    }
                    Err(e)
                        if e.kind() == io::ErrorKind::WouldBlock
                            || e.kind() == io::ErrorKind::TimedOut =>
                    {
                        thread::sleep(Duration::from_millis(READ_RETRY_SLEEP_MS));
                    }
                    Err(e) => {
                        let source_name = source_info
                            .as_ref()
                            .map(|(_, n)| n.clone())
                            .unwrap_or_else(|| "unknown".to_string());

                        log::error!("Driver read error on {}: {}", source_name, e);
                        let _ = can_sender.send(CanMessage::ConnectionFailed {
                            source: source_name,
                            error: e.to_string(),
                        });
                        driver = None; // Drop driver, will retry on next loop iteration
                        thread::sleep(Duration::from_millis(RECONNECT_DELAY_MS));
                    }
                }
            }
        }
    })
}

fn load_parser(path: Option<PathBuf>) -> Option<can_decode::Parser> {
    path.and_then(|p| match can_decode::Parser::from_dbc_file(&p) {
        Ok(parser) => {
            log::info!("Worker loaded DBC: {:?}", p);
            Some(parser)
        }
        Err(e) => {
            log::error!("Worker failed to load DBC: {e}");
            None
        }
    })
}

fn process_frame(
    can_sender: &mpsc::Sender<CanMessage>,
    parser: &mut Option<can_decode::Parser>,
    frame: CanFrame,
) {
    match frame {
        CanFrame::Can2(frame2) => {
            let id = match frame2.id() {
                slcan::Id::Standard(sid) => sid.as_raw() as u32,
                slcan::Id::Extended(eid) => eid.as_raw(),
            };

            let data = frame2.data().unwrap_or(&[]);

            if let Some(p) = parser.as_ref() {
                if let Some(decoded) = p.decode_msg(id, data) {
                    let parsed_msg = ParsedMessage {
                        timestamp: Local::now(),
                        raw_bytes: data.to_vec(),
                        decoded,
                    };
                    let _ = can_sender.send(CanMessage::ParsedMessage(parsed_msg));
                }
            }
        }
        CanFrame::CanFd(frame_fd) => {
            log::info!("Received CAN FD frame: {:?}", frame_fd);
        }
    }
}
