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

fn stream_decoded<'a>(
    parser: &'a can_decode::Parser,
    logs_dir: std::path::PathBuf,
) -> impl Iterator<Item = LogMessage> + 'a {
    let mut file_paths = std::fs::read_dir(logs_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("log")
        })
        .collect::<Vec<_>>();
    file_paths.sort();

    // Create an iterator over files, then flatten each file's iterator of parsed messages
    file_paths.into_iter().flat_map(move |path| {
        let file = std::fs::File::open(&path).expect("Failed to open log file");
        let mut reader = std::io::BufReader::new(file);
        let mut buff = [0u8; logs::consts::MSG_BYTE_LEN];

        std::iter::from_fn(move || {
            let bytes_read = match reader.read(&mut buff) {
                Ok(0) => return None, // EOF
                Ok(n) => n,
                Err(e) => {
                    eprintln!("Error reading log file {:?}: {}", path, e);
                    return None;
                }
            };

            if bytes_read < logs::consts::MSG_BYTE_LEN {
                eprintln!(
                    "Incomplete message in log file {:?}: expected {} bytes, got {}",
                    path,
                    logs::consts::MSG_BYTE_LEN,
                    bytes_read
                );
                return None;
            }

            let timestamp = u32::from_le_bytes(
                buff[logs::consts::FRAME_TYPE_OFFSET..logs::consts::TIMESTAMP_OFFSET]
                    .try_into()
                    .unwrap(),
            );
            let logged_id = u32::from_le_bytes(
                buff[logs::consts::TIMESTAMP_OFFSET..logs::consts::ID_OFFSET]
                    .try_into()
                    .unwrap(),
            );
            let dlc = buff[logs::consts::DLC_OFFSET];
            let data = &buff[logs::consts::DATA_OFFSET..logs::consts::DATA_OFFSET + dlc as usize];

            let is_extended = (logged_id & logs::consts::CAN_EFF_FLAG) != 0;
            let arb_id = if is_extended {
                logged_id & logs::consts::CAN_EXT_ID_MASK
            } else {
                logged_id & logs::consts::CAN_STD_ID_MASK
            };

            match parser.decode_msg(arb_id, data) {
                Some(decoded) => Some(LogMessage { timestamp, decoded }),
                None => {
                    eprintln!(
                        "Failed to decode message with ID 0x{:X} in log file {:?}",
                        arb_id, path
                    );
                    None
                }
            }
        })
    })
}
