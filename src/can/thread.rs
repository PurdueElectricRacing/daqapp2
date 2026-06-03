use crate::daq_log_parse::consts::{BUS_ID_MASK, IS_EID_MASK};
use crate::daq_log_parse::parse::RawFrame;
use crate::util::byte_to_bcd_format;
use crate::util::get_absolute_path_to;
use crate::{can, connection, messages, util};
use crate::daq_log_parse::consts::{NO_CONNECTION_SLEEP_MS, READ_RETRY_SLEEP_MS, BUS_LOAD_UPDATE_MS, LOG_FRAMES_MS, LOG_FOLDER_PATH};

use chrono::{Datelike, Timelike};
use std::fs::{File, create_dir_all};
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

pub struct DaqLogger {
    pub(crate) file: Option<File>,
    pub(crate) folder_path: PathBuf,
    pub(crate) buffer: Vec<RawFrame>,
    pub(crate) file_created_at: Instant,
    pub(crate) start_time: Instant,
    pub(crate) last_flush: Instant,
    pub(crate) buffer_capacity: usize,
}

impl DaqLogger {
    pub fn new() -> Self {
        let path = get_absolute_path_to(LOG_FOLDER_PATH);
        create_dir_all(&path).expect("Failed to create logs directory");

        Self {
            file: None,
            folder_path: path,
            buffer: Vec::with_capacity(10000),
            file_created_at: Instant::now(),
            start_time: Instant::now(),
            last_flush: Instant::now(),
            buffer_capacity: 5000,
        }
    }

    pub fn log_frame(&mut self, frame: &slcan::Can2Frame, bus_id: u8) {
        let (id, data) = match frame.id() {
            slcan::Id::Standard(sid) => {
                let id = sid.as_raw() as u32;
                (id, frame.data().unwrap_or(&[]))
            }
            slcan::Id::Extended(eid) => {
                let id = eid.as_raw() | IS_EID_MASK;
                (id, frame.data().unwrap_or(&[]))
            }
        };

        let frame_identity = if bus_id != 0 { id | BUS_ID_MASK } else { id };

        let mut data_array = [0u8; 8];
        data_array[..data.len().min(8)].copy_from_slice(&data[..data.len().min(8)]);

        let ticks_ms = self.start_time.elapsed().as_millis() as u32;

        let raw_frame = RawFrame {
            ticks_ms: ticks_ms,
            identity: frame_identity,
            data: data_array,
        };

        self.add_frame(raw_frame);
    }

    fn add_frame(&mut self, frame: RawFrame) {
        self.buffer.push(frame);

        //Flush every 1 second
        if self.buffer.len() >= self.buffer_capacity || self.last_flush.elapsed().as_millis() >= 1000 {
            self.flush();
        }
    }

    pub fn flush(&mut self) {
        if self.buffer.is_empty() {
            return;
        }

        // Create new file if time of creation has exceed threshold
        if self.file.is_some() && self.file_created_at.elapsed().as_millis() >= LOG_FRAMES_MS {
            self.file = None;
        }

        if self.file.is_none() {
            let now = chrono::Local::now();
            self.file_created_at = Instant::now();

            let year_bcd = byte_to_bcd_format((now.year() % 100) as u8);
            let month_bcd = byte_to_bcd_format(now.month() as u8);
            let day_bcd = byte_to_bcd_format(now.day() as u8);
            let hour_bcd = byte_to_bcd_format(now.hour() as u8);
            let min_bcd = byte_to_bcd_format(now.minute() as u8);
            let sec_bcd = byte_to_bcd_format(now.second() as u8);

            let filename = format!(
                "log-20{:02x}-{:02x}-{:02x}--{:02x}-{:02x}-{:02x}.log",
                year_bcd, month_bcd, day_bcd, hour_bcd, min_bcd, sec_bcd
            );

            let file_path = self.folder_path.join(filename);
            self.file = Some(File::create(file_path).expect("Failed to create log file"));
        }

        if let Some(ref mut file) = self.file {
            for frame in &self.buffer {
                if let Err(e) = file.write_all(bytemuck::bytes_of(frame)) {
                    log::error!("Failed to write to log file: {}", e);
                    break;
                }
            }

            if let Err(e) = file.flush() {
                log::error!("Failed to flush log file: {}", e);
            }
        }

        self.buffer.clear();
        self.last_flush = Instant::now();
    }

    pub fn shutdown(&mut self) {
        self.flush();
        if let Some(ref mut file) = self.file.take() {
            let _ = file.sync_all();
        }
    }
}

impl Drop for DaqLogger {
    fn drop(&mut self) {
        self.shutdown();
    }
}

// Returns the number of payload data bytes in the CAN frame if it was a Can2 frame
fn process_can_frame(frame: slcan::CanFrame, state: &can::state::State) -> usize {
    match frame {
        slcan::CanFrame::Can2(frame2) => {
            let decode_msg_id = util::can::slcan_to_u32_with_extid_flag(&frame2.id());
            let raw_msg_id = util::can::slcan_to_u32_without_extid_flag(&frame2.id());

            let data = frame2.data().unwrap_or(&[]);
            let timestamp = chrono::Local::now();
            let raw_bytes = data.to_vec();

            let decoded = state
                .parser
                .as_ref()
                .and_then(|parser| parser.decode_msg(decode_msg_id, data));

            match decoded {
                Some(decoded) => {
                    let parsed_msg = messages::ParsedMessage {
                        timestamp,
                        raw_bytes,
                        decoded,
                    };
                    state
                        .can_to_ui_tx
                        .send(messages::MsgFromCan::ParsedMessage(parsed_msg))
                        .expect("Failed to send parsed CAN message");
                }
                None => {
                    if state.parser.is_some() {
                        log::error!(
                            "Failed to parse: frame ID 0x{:X} ({}), data: {:02X?}",
                            raw_msg_id,
                            raw_msg_id,
                            data
                        );
                    } else {
                        log::warn!(
                            "No DBC loaded. Received frame ID 0x{:X} ({}), data: {:02X?}",
                            raw_msg_id,
                            raw_msg_id,
                            data
                        );
                    }

                    let unparsed_msg = messages::UnparsedMessage {
                        timestamp,
                        raw_bytes,
                        msg_id: raw_msg_id,
                    };
                    state
                        .can_to_ui_tx
                        .send(messages::MsgFromCan::UnparsedMessage(unparsed_msg))
                        .expect("Failed to send unparsed CAN message");
                }
            }

            data.len()
        }

        slcan::CanFrame::CanFd(frame_fd) => {
            let msg_id_raw = util::can::slcan_to_u32_without_extid_flag(&frame_fd.id());
            log::warn!(
                "Received CAN FD frame id=0x{:X} len={}",
                msg_id_raw,
                frame_fd.data().len()
            );
            frame_fd.data().len()
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
        let mut daq_logger = DaqLogger::new();

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
                        slcan::ExtendedId::new(msg.msg_id & util::can::EXTENDED_ID_MASK)
                            .map(slcan::Id::Extended)
                    } else if msg.msg_id <= util::can::STANDARD_ID_MASK {
                        slcan::StandardId::new(msg.msg_id as u16).map(slcan::Id::Standard)
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

            match active_driver.read_frames() {
                Ok(frames) => {
                    for frame in frames {
                        let data_bytes = process_can_frame(frame.clone(), &state);
                        state.bus_load_tracker.record_frame(data_bytes);

                        match &frame {
                            slcan::CanFrame::Can2(f2) => daq_logger.log_frame(f2, 0),
                            slcan::CanFrame::CanFd(_) => log::error!("CAN FD Message Could Not Be Logged"),
                        }
                    }

                    // Send bus load updates periodically
                    if state.last_bus_load_update.elapsed().as_millis() >= BUS_LOAD_UPDATE_MS {
                        state.bus_load_tracker.cleanup();
                        let can_bus_speed = state
                            .driver
                            .as_ref()
                            .and_then(|d| d.bus_speed())
                            .unwrap_or_default();
                        let load_1s = state.bus_load_tracker.get_load(1, can_bus_speed);
                        let load_5s = state.bus_load_tracker.get_load(5, can_bus_speed);
                        let load_10s = state.bus_load_tracker.get_load(10, can_bus_speed);
                        let load_30s = state.bus_load_tracker.get_load(30, can_bus_speed);

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