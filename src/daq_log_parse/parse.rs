use crate::daq_log_parse::consts;
use bytemuck::{Pod, Zeroable};


#[derive(Debug)]
pub struct ParsedMessage {
    pub timestamp: u32,
    pub decoded: can_decode::DecodedMessage,
}

#[repr(C)]
#[derive(Pod, Zeroable, Copy, Clone)]
// based on definition of timestamped_frame_t in spmc.h in firmware repo
struct RawFrame {
    ticks_ms: u32,
    identity: u32,
    data: [u8; 8],
}

pub fn parse_log_files(
    in_folder: &std::path::Path,
    parser: &can_decode::Parser,
) -> Vec<ParsedMessage> {
    let mut all_parsed = Vec::new();
    let mut file_paths = std::fs::read_dir(in_folder)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("log")
        })
        .collect::<Vec<_>>();
    file_paths.sort();
    for path in file_paths {
        println!("Parsing log file: {}", path.display());
        let parsed = parse_log_file(&path, parser);
        all_parsed.extend(parsed);
    }

    all_parsed
}

fn parse_log_file(in_file: &std::path::Path, parser: &can_decode::Parser) -> Vec<ParsedMessage> {
    let content = std::fs::read(in_file).unwrap();

    // TODO: handle case here where log format might be outdated, don't assume cast_slice will work
    let frames = bytemuck::try_cast_slice::<u8, RawFrame>(&content).expect("Failed to parse log file - possibly due to outdated log format");
    let mut parsed = Vec::with_capacity(frames.len());

    for frame in frames {
        let arb_id = if (frame.identity & consts::IS_EID_MASK) != 0 {
            frame.identity & consts::CAN_EID_MASK
        } else {
            frame.identity & consts::CAN_STD_ID_MASK
        };

        if let Some(decoded) = parser.decode_msg(arb_id, &frame.data) {
            parsed.push(ParsedMessage {
                timestamp: frame.ticks_ms,
                decoded,
            });
        } else {
            log::error!(
                "Failed to decode message at {} ms with CAN ID {:X} and data {:?}",
                frame.ticks_ms,
                arb_id,
                frame.data,
            );
        }
    }
    parsed
}

pub fn chunk_parsed(parsed: Vec<ParsedMessage>) -> Vec<Vec<ParsedMessage>> {
    let mut chunks = Vec::new();
    let mut current_chunk = Vec::new();
    let mut last_timestamp = None;

    for msg in parsed {
        if let Some(last_ts) = last_timestamp
            && (msg.timestamp < last_ts || msg.timestamp - last_ts > consts::MAX_JUMP_MS)
            && !current_chunk.is_empty()
        {
            chunks.push(current_chunk);
            current_chunk = Vec::new();
        }
        last_timestamp = Some(msg.timestamp);
        current_chunk.push(msg);
    }
    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    // Sort messages within each chunk by timestamp
    for chunk in &mut chunks {
        chunk.sort_by_key(|m| m.timestamp);
    }

    chunks
}
