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

enum State {
    Idle,
    Connecting {
        source: ConnectionSource,
        name: String,
    },
    Connected {
        driver: crate::can::Driver,
        source: ConnectionSource,
        name: String,
    },
}

struct Worker {
    can_sender: mpsc::Sender<CanMessage>,
    command_receiver: mpsc::Receiver<WorkerCommand>,
    parser: Option<can_decode::Parser>,
    state: State,
}

pub fn spawn_worker(
    can_sender: mpsc::Sender<CanMessage>,
    command_receiver: mpsc::Receiver<WorkerCommand>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut worker = Worker {
            can_sender,
            command_receiver,
            parser: None,
            state: State::Idle,
        };
        worker.run();
    })
}

impl Worker {
    fn run(&mut self) {
        log::info!("CAN persistent worker started");

        loop {
            // 1. Handle commands
            let cmd = match &self.state {
                State::Idle => match self.command_receiver.recv() {
                    Ok(c) => Some(c),
                    Err(_) => {
                        log::info!("CAN worker command channel disconnected, shutting down");
                        break;
                    }
                },
                _ => self.command_receiver.try_recv().ok(),
            };

            if let Some(c) = cmd {
                if !self.handle_command(c) {
                    break;
                }
            }

            // 2. Run state logic
            self.step();
        }
    }

    fn handle_command(&mut self, cmd: WorkerCommand) -> bool {
        match cmd {
            WorkerCommand::Shutdown => {
                log::info!("CAN worker received Shutdown");
                false
            }
            WorkerCommand::Disconnect => {
                log::info!("CAN worker Disconnect");
                self.state = State::Idle;
                true
            }
            WorkerCommand::UpdateDbc(path) => {
                self.parser = load_parser(path);
                true
            }
            WorkerCommand::Connect { source, dbc_path } => {
                let name = match &source {
                    ConnectionSource::Serial(p) => p.clone(),
                    ConnectionSource::Udp(p) => format!("UDP:{}", p),
                };
                log::info!("CAN worker connecting to {}", name);
                self.parser = load_parser(dbc_path);
                self.state = State::Connecting { source, name };
                true
            }
        }
    }

    fn step(&mut self) {
        match std::mem::replace(&mut self.state, State::Idle) {
            State::Idle => {}
            State::Connecting { source, name } => match source.create_driver() {
                Ok(driver) => {
                    let _ = self.can_sender.send(CanMessage::ConnectionSuccessful);
                    self.state = State::Connected {
                        driver,
                        source,
                        name,
                    };
                }
                Err(e) => {
                    let _ = self.can_sender.send(CanMessage::ConnectionFailed {
                        source: name.clone(),
                        error: e.to_string(),
                    });
                    thread::sleep(Duration::from_millis(RECONNECT_DELAY_MS));
                    self.state = State::Connecting { source, name };
                }
            },
            State::Connected {
                mut driver,
                source,
                name,
            } => match driver.read_frame() {
                Ok(frame) => {
                    process_frame(&self.can_sender, &mut self.parser, frame);
                    self.state = State::Connected {
                        driver,
                        source,
                        name,
                    };
                }
                Err(e)
                    if e.kind() == io::ErrorKind::WouldBlock
                        || e.kind() == io::ErrorKind::TimedOut =>
                {
                    thread::sleep(Duration::from_millis(READ_RETRY_SLEEP_MS));
                    self.state = State::Connected {
                        driver,
                        source,
                        name,
                    };
                }
                Err(e) => {
                    log::error!("Driver read error on {}: {}", name, e);
                    let _ = self.can_sender.send(CanMessage::ConnectionFailed {
                        source: name.clone(),
                        error: e.to_string(),
                    });
                    thread::sleep(Duration::from_millis(RECONNECT_DELAY_MS));
                    self.state = State::Connecting { source, name };
                }
            },
        }
    }
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
