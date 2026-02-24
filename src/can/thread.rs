use crate::can::{can_messages::CanMessage, message::ParsedMessage, ConnectionSource};
use chrono::Local;
use slcan::CanFrame;
use std::{
    io,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

const READ_RETRY_SLEEP_MS: u64 = 2;
const RECONNECT_DELAY_MS: u64 = 1000;

pub fn spawn_worker(
    can_sender: mpsc::Sender<CanMessage>,
    source: ConnectionSource,
    dbc_path: Option<PathBuf>,
    stop_signal: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut parser = dbc_path.and_then(|path| match can_decode::Parser::from_dbc_file(&path) {
            Ok(p) => {
                log::info!("Worker loaded DBC: {:?}", path);
                Some(p)
            }
            Err(e) => {
                log::error!("Worker failed to load DBC: {e}");
                None
            }
        });

        let source_name = match &source {
            ConnectionSource::Serial(p) => p.clone(),
            ConnectionSource::Udp(p) => format!("UDP:{}", p),
        };

        log::info!("CAN worker started for {}", source_name);

        'outer: while !stop_signal.load(Ordering::Relaxed) {
            let mut driver = match source.create_driver() {
                Ok(d) => d,
                Err(e) => {
                    let _ = can_sender.send(CanMessage::ConnectionFailed {
                        source: source_name.clone(),
                        error: e.to_string(),
                    });
                    thread::sleep(Duration::from_millis(RECONNECT_DELAY_MS));
                    continue 'outer;
                }
            };

            let mut first_success = true;

            while !stop_signal.load(Ordering::Relaxed) {
                // Read from driver
                match driver.read_frame() {
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
                        log::error!("Driver read error on {}: {}", source_name, e);
                        let _ = can_sender.send(CanMessage::ConnectionFailed {
                            source: source_name.clone(),
                            error: e.to_string(),
                        });
                        thread::sleep(Duration::from_millis(RECONNECT_DELAY_MS));
                        continue 'outer;
                    }
                }
            }
        }
        log::info!("CAN worker stopped via signal");
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
