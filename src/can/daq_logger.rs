use crate::daq_log_parse::consts::{BUS_ID_MASK, IS_EID_MASK};

use crate::daq_log_parse::parse::RawFrame;
use crate::util::get_absolute_path_to;

use chrono::{Datelike, Timelike};
use std::fs::{File, create_dir_all};
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

pub const LOG_FRAMES_MS: u128 = 60000;
pub const LOG_FOLDER_PATH: &str = "logs";

pub fn byte_to_bcd_format(val: u8) -> u8 {
    ((val / 10) << 4) | (val % 10)
}

pub struct DaqLogger {
    file: Option<File>,
    folder_path: PathBuf,
    buffer: Vec<RawFrame>,
    file_created_at: Instant,
    start_time: Instant,
    last_flush: Instant,
    buffer_capacity: usize,
}

impl DaqLogger {
    pub fn new(folder_path: Option<std::path::PathBuf>) -> Self {
        let path = folder_path.unwrap_or_else(|| get_absolute_path_to(LOG_FOLDER_PATH));

        if let Err(e) = create_dir_all(&path) {
            log::error!("Failed to create directory for logs: {:?}: {}", path, e);
        }

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
            ticks_ms,
            identity: frame_identity,
            data: data_array,
        };

        self.add_frame(raw_frame);
    }

    fn add_frame(&mut self, frame: RawFrame) {
        self.buffer.push(frame);

        //Flush every 1 second
        if self.buffer.len() >= self.buffer_capacity
            || self.last_flush.elapsed().as_millis() >= 1000
        {
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
            match File::create(&file_path) {
                Ok(f) => self.file = Some(f),
                Err(e) => {
                    log::error!("Failed to create log file {:?}: {}", file_path, e);
                    self.buffer.clear();
                    self.last_flush = Instant::now();
                    return;
                }
            }
        }

        if let Some(ref mut file) = self.file {
            if let Err(e) = file.write_all(bytemuck::cast_slice(&self.buffer)) {
                log::error!("Failed to write to log file: {}", e);
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
