use std::io::Read;

use crate::logs;

pub struct LogMessage {
    pub timestamp: u32,
    pub decoded: can_decode::DecodedMessage,
}

pub fn start_log_parse_thread(
    dbc_path: std::path::PathBuf,
    logs_dir: std::path::PathBuf,
    output_dir: std::path::PathBuf,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let parser = match can_decode::Parser::from_dbc_file(&dbc_path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to load DBC from {:?}: {}", dbc_path, e);
                return;
            }
        };
    })
}

