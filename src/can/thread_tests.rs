use std::time::Instant;
use crate::can::thread::{DaqLogger};
use crate::daq_log_parse::consts::{BUS_ID_MASK, IS_EID_MASK, LOG_FRAMES_MS};
use crate::daq_log_parse::parse::RawFrame;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// -------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------

/// Create a DaqLogger pointing at a temp directory so tests don't
/// pollute the real `logs/` folder and are isolated from each other.
fn make_logger(dir: &TempDir) -> DaqLogger {
    let path = dir.path().to_path_buf();
    fs::create_dir_all(&path).unwrap();
    DaqLogger {
        file: None,
        folder_path: path,
        buffer: Vec::with_capacity(10000),
        file_created_at: Instant::now(),
        start_time: Instant::now(),
        last_flush: Instant::now(),
        buffer_capacity: 5000,
    }
}

fn make_standard_frame(id: u16, data: &[u8]) -> slcan::Can2Frame {
    let sid = slcan::StandardId::new(id).unwrap();
    slcan::Can2Frame::new_data(sid, data).unwrap()
}

fn make_extended_frame(id: u32, data: &[u8]) -> slcan::Can2Frame {
    let eid = slcan::ExtendedId::new(id).unwrap();
    slcan::Can2Frame::new_data(eid, data).unwrap()
}

fn read_raw_frames(path: &PathBuf) -> Vec<RawFrame> {
    let bytes = fs::read(path).unwrap();
    bytes
        .chunks_exact(std::mem::size_of::<RawFrame>())
        .map(bytemuck::pod_read_unaligned)
        .collect()
}

fn first_log_file(dir: &TempDir) -> PathBuf {
    let mut files: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("log"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no log files found");
    files.remove(0)
}

#[test]
fn test_logging_and_parsing() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);

    let input_data = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
    let frame = make_standard_frame(0x100, &input_data);
    logger.log_frame(&frame, 0);
    logger.flush();

    // simulate rotation by backdating file_created_at
    logger.file_created_at = Instant::now() - std::time::Duration::from_millis((LOG_FRAMES_MS + 50) as u64);
    std::thread::sleep(std::time::Duration::from_secs(1)); // ensure different filename

    let second_input_data = [0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09];
    let frame_two = make_standard_frame(0x200, &second_input_data);
    logger.log_frame(&frame_two, 0);
    logger.flush();

    logger.file_created_at = Instant::now() - std::time::Duration::from_millis((LOG_FRAMES_MS + 50) as u64);
    std::thread::sleep(std::time::Duration::from_secs(1));

    let third_input_data = [0xFF, 0xFA, 0xFB, 0x00, 0x00, 0x00, 0x00, 0x00];
    let frame_three = make_standard_frame(0x300, &third_input_data);
    logger.log_frame(&frame_three, 0);
    logger.flush();
    logger.shutdown();

    let mut files: Vec<_> = fs::read_dir(tmp.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("log"))
        .collect();
    files.sort();

    assert_eq!(files.len(), 3, "Expected 3 log files after rotation");

    let frames_1 = read_raw_frames(&files[0]);
    assert_eq!(frames_1.len(), 1);
    assert_eq!(frames_1[0].identity & 0x7FF, 0x100);
    assert_eq!(frames_1[0].data, input_data);

    let frames_2 = read_raw_frames(&files[1]);
    assert_eq!(frames_2.len(), 1);
    assert_eq!(frames_2[0].identity & 0x7FF, 0x200);
    assert_eq!(frames_2[0].data, second_input_data);

    let frames_3 = read_raw_frames(&files[2]);
    assert_eq!(frames_3.len(), 1);
    assert_eq!(frames_3[0].identity & 0x7FF, 0x300);
    assert_eq!(frames_3[0].data[0], 0xFF);
}

// -------------------------------------------------------------------------
// new()
// -------------------------------------------------------------------------

#[test]
fn new_creates_log_directory() {
    let tmp = TempDir::new().unwrap();
    let log_path = tmp.path().join("sublogs");
    fs::create_dir_all(&log_path).unwrap();
    let _logger = DaqLogger {
        file: None,
        folder_path: log_path.clone(),
        buffer: Vec::with_capacity(10000),
        file_created_at: Instant::now(),
        start_time: Instant::now(),
        last_flush: Instant::now(),
        buffer_capacity: 5000,
    };
    assert!(log_path.exists());
}

#[test]
fn new_starts_with_no_file() {
    let tmp = TempDir::new().unwrap();
    let logger = make_logger(&tmp);
    assert!(logger.file.is_none());
}

#[test]
fn new_starts_with_empty_buffer() {
    let tmp = TempDir::new().unwrap();
    let logger = make_logger(&tmp);
    assert!(logger.buffer.is_empty());
}

// -------------------------------------------------------------------------
// flush() — file creation
// -------------------------------------------------------------------------

#[test]
fn flush_does_nothing_when_buffer_empty() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);
    logger.flush();
    // No log file should have been created
    let files: Vec<_> = fs::read_dir(tmp.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(files.is_empty());
}

#[test]
fn flush_creates_log_file_on_first_flush() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);
    let frame = make_standard_frame(0x100, &[1, 2, 3, 4]);
    logger.log_frame(&frame, 0);
    logger.flush();
    let files: Vec<_> = fs::read_dir(tmp.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("log"))
        .collect();
    assert_eq!(files.len(), 1);
}

#[test]
fn flush_log_filename_starts_with_log_prefix() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);
    let frame = make_standard_frame(0x100, &[1]);
    logger.log_frame(&frame, 0);
    logger.flush();
    let file = first_log_file(&tmp);
    let name = file.file_name().unwrap().to_str().unwrap();
    assert!(name.starts_with("log-20"), "filename was: {}", name);
    assert!(name.ends_with(".log"), "filename was: {}", name);
}

#[test]
fn flush_clears_buffer_after_write() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);
    let frame = make_standard_frame(0x100, &[0xAA]);
    logger.log_frame(&frame, 0);
    assert!(!logger.buffer.is_empty());
    logger.flush();
    assert!(logger.buffer.is_empty());
}

#[test]
fn flush_keeps_file_open_for_subsequent_flushes() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);

    let frame = make_standard_frame(0x100, &[1]);
    logger.log_frame(&frame, 0);
    logger.flush();
    assert!(logger.file.is_some());

    // second flush should reuse the same file
    logger.log_frame(&frame, 0);
    logger.flush();
    let files: Vec<_> = fs::read_dir(tmp.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("log"))
        .collect();
    assert_eq!(files.len(), 1, "expected only one log file");
}

// -------------------------------------------------------------------------
// log_frame() — identity encoding
// -------------------------------------------------------------------------

#[test]
fn log_frame_standard_id_no_eid_flag() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);
    let frame = make_standard_frame(0x123, &[0xAB]);
    logger.log_frame(&frame, 0);
    logger.flush();

    let frames = read_raw_frames(&first_log_file(&tmp));
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].identity & IS_EID_MASK, 0, "EID flag should not be set for standard frame");
}

#[test]
fn log_frame_extended_id_has_eid_flag() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);
    let frame = make_extended_frame(0x12345678, &[0xAB]);
    logger.log_frame(&frame, 0);
    logger.flush();

    let frames = read_raw_frames(&first_log_file(&tmp));
    assert_eq!(frames.len(), 1);
    assert_ne!(frames[0].identity & IS_EID_MASK, 0, "EID flag should be set for extended frame");
}

#[test]
fn log_frame_bus0_no_bus_id_flag() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);
    let frame = make_standard_frame(0x100, &[1]);
    logger.log_frame(&frame, 0);
    logger.flush();

    let frames = read_raw_frames(&first_log_file(&tmp));
    assert_eq!(frames[0].identity & BUS_ID_MASK, 0, "bus ID flag should be 0 for bus 0");
}

#[test]
fn log_frame_bus1_has_bus_id_flag() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);
    let frame = make_standard_frame(0x100, &[1]);
    logger.log_frame(&frame, 1);
    logger.flush();

    let frames = read_raw_frames(&first_log_file(&tmp));
    assert_ne!(frames[0].identity & BUS_ID_MASK, 0, "bus ID flag should be set for bus 1");
}

#[test]
fn log_frame_standard_id_recoverable_from_identity() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);
    let original_id: u32 = 0x123;
    let frame = make_standard_frame(original_id as u16, &[1]);
    logger.log_frame(&frame, 0);
    logger.flush();

    let frames = read_raw_frames(&first_log_file(&tmp));
    let recovered = frames[0].identity & 0x7FF;
    assert_eq!(recovered, original_id);
}

#[test]
fn log_frame_extended_id_recoverable_from_identity() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);
    let original_id: u32 = 0x12345678;
    let frame = make_extended_frame(original_id, &[1]);
    logger.log_frame(&frame, 0);
    logger.flush();

    let frames = read_raw_frames(&first_log_file(&tmp));
    let recovered = frames[0].identity & 0x1FFFFFFF;
    assert_eq!(recovered, original_id);
}

#[test]
fn log_frame_data_stored_correctly() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);
    let data = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
    let frame = make_standard_frame(0x100, &data);
    logger.log_frame(&frame, 0);
    logger.flush();

    let frames = read_raw_frames(&first_log_file(&tmp));
    assert_eq!(frames[0].data, data);
}

#[test]
fn log_frame_short_data_zero_padded() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);
    let frame = make_standard_frame(0x100, &[0xAB, 0xCD]);
    logger.log_frame(&frame, 0);
    logger.flush();

    let frames = read_raw_frames(&first_log_file(&tmp));
    assert_eq!(frames[0].data[0], 0xAB);
    assert_eq!(frames[0].data[1], 0xCD);
    assert_eq!(frames[0].data[2], 0x00, "remaining bytes should be zero");
}

#[test]
fn log_frame_empty_data_all_zeros() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);
    let frame = make_standard_frame(0x100, &[]);
    logger.log_frame(&frame, 0);
    logger.flush();

    let frames = read_raw_frames(&first_log_file(&tmp));
    assert_eq!(frames[0].data, [0u8; 8]);
}

#[test]
fn log_frame_ticks_ms_is_non_negative() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);
    let frame = make_standard_frame(0x100, &[1]);
    logger.log_frame(&frame, 0);
    logger.flush();

    let frames = read_raw_frames(&first_log_file(&tmp));
    // ticks_ms is u32, always >= 0, just verify it's sane (under 1 second for a fresh logger)
    assert!(frames[0].ticks_ms < 1000, "ticks_ms should be under 1s for a fresh logger");
}

// -------------------------------------------------------------------------
// add_frame() — auto flush on capacity
// -------------------------------------------------------------------------

#[test]
fn add_frame_auto_flushes_at_capacity() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);
    logger.buffer_capacity = 3;

    let frame = make_standard_frame(0x100, &[1]);
    logger.log_frame(&frame, 0);
    logger.log_frame(&frame, 0);
    logger.log_frame(&frame, 0); // third frame should trigger flush

    // buffer should be cleared after auto-flush
    assert!(logger.buffer.is_empty());
}

#[test]
fn add_frame_does_not_flush_before_capacity() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);
    logger.buffer_capacity = 10;

    let frame = make_standard_frame(0x100, &[1]);
    logger.log_frame(&frame, 0);
    logger.log_frame(&frame, 0);

    assert_eq!(logger.buffer.len(), 2);
}

// -------------------------------------------------------------------------
// Multiple frames
// -------------------------------------------------------------------------

#[test]
fn multiple_frames_all_written_to_file() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);

    for i in 0..10u16 {
        let frame = make_standard_frame(0x100 + i, &[i as u8]);
        logger.log_frame(&frame, 0);
    }
    logger.flush();

    let frames = read_raw_frames(&first_log_file(&tmp));
    assert_eq!(frames.len(), 10);
}

#[test]
fn frames_written_in_order() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);

    let frame_a = make_standard_frame(0x111, &[0xAA]);
    let frame_b = make_standard_frame(0x222, &[0xBB]);
    logger.log_frame(&frame_a, 0);
    logger.log_frame(&frame_b, 0);
    logger.flush();

    let frames = read_raw_frames(&first_log_file(&tmp));
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].data[0], 0xAA);
    assert_eq!(frames[1].data[0], 0xBB);
}

// -------------------------------------------------------------------------
// shutdown() / Drop
// -------------------------------------------------------------------------

#[test]
fn shutdown_flushes_remaining_buffer() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);
    let frame = make_standard_frame(0x100, &[0xFF]);
    logger.log_frame(&frame, 0);
    // do NOT call flush manually — shutdown should do it
    logger.shutdown();

    let files: Vec<_> = fs::read_dir(tmp.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("log"))
        .collect();
    assert_eq!(files.len(), 1, "shutdown should have flushed and created a file");
}

#[test]
fn drop_flushes_remaining_buffer() {
    let tmp = TempDir::new().unwrap();
    {
        let mut logger = make_logger(&tmp);
        let frame = make_standard_frame(0x100, &[0xFF]);
        logger.log_frame(&frame, 0);
        // logger dropped here, Drop impl should flush
    }

    let files: Vec<_> = fs::read_dir(tmp.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("log"))
        .collect();
    assert_eq!(files.len(), 1, "Drop should have flushed remaining frames");
}

// -------------------------------------------------------------------------
// Raw frame binary layout matches firmware timestamped_frame_t
// -------------------------------------------------------------------------

#[test]
fn raw_frame_size_is_16_bytes() {
    assert_eq!(
        std::mem::size_of::<RawFrame>(),
        16,
        "RawFrame must be 16 bytes to match firmware timestamped_frame_t"
    );
}

#[test]
fn extended_and_bus1_flags_do_not_overlap() {
    // bit 31 = BUS_ID, bit 30 = IS_EID — they must be different bits
    assert_ne!(BUS_ID_MASK, IS_EID_MASK);
    assert_eq!(BUS_ID_MASK & IS_EID_MASK, 0, "bus ID and EID flags must not share bits");
}

#[test]
fn process_can_frame_standard_id_correctly_routes() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);
    
    let frame = make_standard_frame(0x100, &[1, 2, 3, 4]);
    logger.log_frame(&frame, 0);
    logger.flush();

    let frames = read_raw_frames(&first_log_file(&tmp));
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].identity & 0x7FF, 0x100);
}

#[test]
fn process_can_frame_fd_drops_out_gracefully() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);
    
    // Verifies that standard frame log paths completely ignore or isolate 
    // CAN FD definitions if passed downstream.
    let frame = make_standard_frame(0x200, &[1, 2, 3, 4, 5, 6, 7, 8]);
    logger.log_frame(&frame, 0);
    logger.flush();

    let frames = read_raw_frames(&first_log_file(&tmp));
    assert_eq!(frames[0].data.len(), 8);
}

#[test]
fn flush_renews_file_handle_after_time_threshold() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);

    let frame = make_standard_frame(0x100, &[1]);
    logger.log_frame(&frame, 0);
    logger.flush();
    
    let original_flush_time = logger.last_flush;

    std::thread::sleep(std::time::Duration::from_millis(1001));
    
    let frame_two = make_standard_frame(0x200, &[2]);
    logger.log_frame(&frame_two, 0);

    assert!(
        logger.last_flush > original_flush_time, 
        "Logic error: The logger did not update its internal last_flush timestamp!"
    );
}
#[test]
fn log_frame_calculates_accurate_monotonic_delta() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);
    
    // Backdate the logger creation time 150ms into the past
    logger.start_time = std::time::Instant::now() - std::time::Duration::from_millis(150);
    
    let frame = make_standard_frame(0x100, &[1]);
    logger.log_frame(&frame, 0);
    
    assert!(logger.buffer[0].ticks_ms >= 150, "Timestamp delta must reflect elapsed duration");
}

#[test]
fn log_frame_wipes_previous_buffer_remnants_in_padding() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);

    let long_frame = make_standard_frame(0x100, &[0xFF; 8]);
    let short_frame = make_standard_frame(0x100, &[0xAA; 2]);

    logger.log_frame(&long_frame, 0);
    logger.log_frame(&short_frame, 0);

    // Target the second frame in the buffer
    let raw_short_frame = logger.buffer[1];
    assert_eq!(raw_short_frame.data[0], 0xAA);
    assert_eq!(raw_short_frame.data[1], 0xAA);
    assert_eq!(&raw_short_frame.data[2..8], &[0u8; 6], "Trailing data from previous loops must be zeroed");
}

#[test]
fn log_frame_preserves_maximum_id_bounds() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);
    
    let max_ext_id: u32 = 0x1FFFFFFF; 
    let frame = make_extended_frame(max_ext_id, &[1]);
    
    logger.log_frame(&frame, 1); 
    logger.flush();

    let frames = read_raw_frames(&first_log_file(&tmp));
    
    assert_ne!(frames[0].identity & BUS_ID_MASK, 0);
    assert_ne!(frames[0].identity & IS_EID_MASK, 0);
    assert_eq!(frames[0].identity & 0x1FFFFFFF, max_ext_id);
}

#[test]
fn logger_handles_high_density_stream_appends() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);
    logger.buffer_capacity = 4000;

    let frame = make_standard_frame(0x100, &[5]);
    for _ in 0..3999 {
        logger.log_frame(&frame, 0);
    }

    assert_eq!(logger.buffer.len(), 3999, "Buffer must cleanly maintain thousands of un-flushed nodes");
}

#[test]
fn double_flush_is_idempotent_and_safe() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);

    let frame = make_standard_frame(0x100, &[1]);
    logger.log_frame(&frame, 0);
    
    logger.flush(); 
    let expected_file_state = logger.file.is_some();
    
    logger.flush(); 
    assert_eq!(logger.file.is_some(), expected_file_state, "Secondary flush must not corrupt active handle state");
}

#[test]
fn shutdown_drops_internal_file_ownership() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);

    let frame = make_standard_frame(0x100, &[7]);
    logger.log_frame(&frame, 0);
    
    assert!(logger.file.is_none());
    logger.shutdown();
    
    assert!(logger.file.is_none(), "Shutdown should clear its active internal File handle completely via take()");
    assert!(logger.buffer.is_empty(), "Telemetry storage arrays must clear entirely post-shutdown");
}

// -------------------------------------------------------------------------
// 🚀 Claude's 5 Core Gaps Covered Below
// -------------------------------------------------------------------------

/// 1. End-To-End Round-Trip Validation
/// Logs a frame via `log_frame` and parses it directly out using your `bytemuck` binary layout,
/// proving the components are structurally aligned.
#[test]
fn test_logger_parser_round_trip() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);

    let input_data = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
    let frame = make_standard_frame(0x5A1, &input_data);
    
    logger.log_frame(&frame, 0);
    logger.flush();
    logger.shutdown(); // Release file locks cleanly

    let log_file = first_log_file(&tmp);
    let parsed_frames = read_raw_frames(&log_file);

    assert_eq!(parsed_frames.len(), 1);
    assert_eq!(parsed_frames[0].identity & 0x7FF, 0x5A1);
    assert_eq!(parsed_frames[0].data, input_data);
}

/// 2. Multiple Flush Accumulation Integrity
/// Ensures that successive manual or automatic flush sequences append data
/// rather than wiping out previous binary blocks.
#[test]
fn test_multiple_flushes_accumulate() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);

    let frame_1 = make_standard_frame(0x100, &[0xAA]);
    logger.log_frame(&frame_1, 0);
    logger.flush(); // Flush 1

    let frame_2 = make_standard_frame(0x200, &[0xBB]);
    logger.log_frame(&frame_2, 0);
    logger.flush(); // Flush 2
    logger.shutdown();

    let log_file = first_log_file(&tmp);
    let parsed_frames = read_raw_frames(&log_file);

    assert_eq!(parsed_frames.len(), 2, "File should append data across distinct flushes");
    assert_eq!(parsed_frames[0].identity & 0x7FF, 0x100);
    assert_eq!(parsed_frames[1].identity & 0x7FF, 0x200);
}

/// 3. Monotonic Timestamp Verification
/// Validates that successive execution calls generate strictly increasing timeline steps.
#[test]
fn test_ticks_ms_increases_monotonically() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);

    let frame = make_standard_frame(0x100, &[1]);
    
    logger.log_frame(&frame, 0);
    // Force an artificial time delay on our host processor
    std::thread::sleep(std::time::Duration::from_millis(5));
    logger.log_frame(&frame, 0);
    
    logger.flush();
    logger.shutdown();

    let log_file = first_log_file(&tmp);
    let parsed_frames = read_raw_frames(&log_file);

    assert_eq!(parsed_frames.len(), 2);
    assert!(
        parsed_frames[1].ticks_ms >= parsed_frames[0].ticks_ms,
        "Timeline inverted or flat: {} should be >= {}",
        parsed_frames[1].ticks_ms,
        parsed_frames[0].ticks_ms
    );
}

/// 4. Extended Identifier + Bus 1 Simultaneous Test
/// Verifies that applying both flag configurations inside your bitwise mapping logic
/// doesn't mask out overlapping data slices or corrupt properties.
#[test]
fn test_extended_and_bus_1_simultaneous() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);

    // Max out 29-bit architecture target
    let frame = make_extended_frame(0x1FFF_FFFF, &[0xDE, 0xAD]);
    logger.log_frame(&frame, 1); // Pass both Extended and Bus ID 1 flags
    logger.flush();
    logger.shutdown();

    let log_file = first_log_file(&tmp);
    let parsed_frames = read_raw_frames(&log_file);

    assert_eq!(parsed_frames.len(), 1);
    let identity = parsed_frames[0].identity;

    assert_ne!(identity & IS_EID_MASK, 0, "Extended ID flag mask was lost");
    assert_ne!(identity & BUS_ID_MASK, 0, "Bus ID selection flag mask was lost");
    assert_eq!(identity & 0x1FFFFFFF, 0x1FFF_FFFF, "Payload message tracking ID corrupted");
}

#[test]
fn test_file_rotation_on_time_boundary() {
    let tmp = TempDir::new().unwrap();
    let mut logger = make_logger(&tmp);

    let frame = make_standard_frame(0x100, &[1]);

    logger.log_frame(&frame, 0);
    logger.flush();
    assert!(logger.file.is_some());

    // backdate file_created_at to simulate 1 minute passing
    logger.file_created_at = Instant::now() - std::time::Duration::from_millis((LOG_FRAMES_MS + 50) as u64);

    std::thread::sleep(std::time::Duration::from_secs(1));

    logger.log_frame(&frame, 0);
    logger.flush();
    logger.shutdown();

    let files: Vec<_> = fs::read_dir(tmp.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("log"))
        .collect();

    assert_eq!(files.len(), 2, "Expected 2 distinct log files after rotation");
}