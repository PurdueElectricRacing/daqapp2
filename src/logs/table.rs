use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::logs;

struct TableHeader {
    bus_row: Vec<String>,
    node_row: Vec<String>,
    message_row: Vec<String>,
    signal_row: Vec<String>,
    indexer: HashMap<(String, String), usize>,
}

impl TableHeader {
    fn new(parser: &can_decode::Parser) -> Self {
        let mut message_defs = parser.msg_defs();
        message_defs.sort_by_key(|m| match m.message_id() {
            can_dbc::MessageId::Standard(id) => *id as u32,
            can_dbc::MessageId::Extended(id) => *id,
        });

        let mut bus_row = vec!["Bus".to_string()];
        let mut node_row = vec!["Node".to_string()];
        let mut message_row = vec!["Message".to_string()];
        let mut signal_row = vec!["Signal".to_string()];
        let mut indexer = HashMap::new();

        let mut col_idx = 1;

        for msg in message_defs {
            let bus_id = "Main";
            let node = match msg.transmitter() {
                can_dbc::Transmitter::NodeName(n) => n,
                can_dbc::Transmitter::VectorXXX => "N/A",
            };
            let msg_name = msg.message_name();

            for sig in msg.signals() {
                let key = (msg_name.to_string(), sig.name().to_string());
                if let std::collections::hash_map::Entry::Vacant(e) = indexer.entry(key) {
                    e.insert(col_idx);
                    col_idx += 1;

                    bus_row.push(bus_id.to_string());
                    node_row.push(node.to_string());
                    message_row.push(msg_name.to_string());
                    signal_row.push(sig.name().to_string());
                }
            }
        }

        Self {
            bus_row,
            node_row,
            message_row,
            signal_row,
            indexer,
        }
    }

    fn write_headers(&self, writer: &mut BufWriter<std::fs::File>) -> std::io::Result<()> {
        write_csv_row(writer, &self.bus_row)?;
        write_csv_row(writer, &self.node_row)?;
        write_csv_row(writer, &self.message_row)?;
        write_csv_row(writer, &self.signal_row)?;
        Ok(())
    }
}

fn write_csv_row(writer: &mut impl Write, cells: &[String]) -> std::io::Result<()> {
    for (i, cell) in cells.iter().enumerate() {
        if i > 0 {
            writer.write_all(b",")?;
        }
        if cell.contains(',') || cell.contains('"') || cell.contains('\n') {
            let escaped = cell.replace('"', "\"\"");
            write!(writer, "\"{}\"", escaped)?;
        } else {
            writer.write_all(cell.as_bytes())?;
        }
    }
    writer.write_all(b"\n")?;
    Ok(())
}

struct OutputFile {
    writer: BufWriter<std::fs::File>,
}

impl OutputFile {
    fn new(output_dir: &Path, counter: usize, header: &TableHeader) -> std::io::Result<Self> {
        println!("Creating output file #{} in {:?}", counter, output_dir);
        let fname = output_dir.join(format!("out_{}.csv", counter));
        let file = std::fs::File::create(&fname)?;
        let mut writer = BufWriter::new(file);
        header.write_headers(&mut writer)?;
        Ok(Self { writer })
    }

    fn write_row(&mut self, row: &[Option<String>]) -> std::io::Result<()> {
        let cells: Vec<String> = row
            .iter()
            .map(|opt| opt.as_deref().unwrap_or("").to_string())
            .collect();
        write_csv_row(&mut self.writer, &cells)
    }
}

struct RowBuilder {
    data: Vec<Option<String>>,
    current_time_ms: u64,
}

impl RowBuilder {
    fn new(num_cols: usize, start_time_ms: u64) -> Self {
        let bin_aligned =
            (start_time_ms / logs::consts::BIN_WIDTH_MS as u64) * logs::consts::BIN_WIDTH_MS as u64;
        let mut data = vec![None; num_cols];
        data[0] = Some(format_time(bin_aligned));

        Self {
            data,
            current_time_ms: bin_aligned,
        }
    }

    fn clear_signals(&mut self) {
        for value in self.data.iter_mut().skip(1) {
            *value = None;
        }
    }

    fn advance_time(&mut self) {
        self.current_time_ms += logs::consts::BIN_WIDTH_MS as u64;
        self.data[0] = Some(format_time(self.current_time_ms));
    }

    fn set_signal(&mut self, col_idx: usize, value: String) {
        if col_idx < self.data.len() {
            self.data[col_idx] = Some(value);
        } else {
            eprintln!(
                "Column index {} out of range for row length {}",
                col_idx,
                self.data.len()
            );
        }
    }

    fn should_advance(&self, timestamp_ms: u64) -> bool {
        timestamp_ms >= self.current_time_ms + logs::consts::BIN_WIDTH_MS as u64
    }
}

fn format_time(time_ms: u64) -> String {
    format!("{:.5}", time_ms as f64 / 1000.0)
}

struct TimingState {
    last_timestamp_ms: Option<u32>,
    initialized: bool,
}

impl TimingState {
    fn new() -> Self {
        Self {
            last_timestamp_ms: None,
            initialized: false,
        }
    }

    fn should_start_new_file(&self, current_timestamp: u32) -> bool {
        !self.initialized
            || self
                .last_timestamp_ms
                .map(|last| {
                    current_timestamp < last
                        || current_timestamp.wrapping_sub(last) >= logs::consts::MAX_JUMP_MS
                })
                .unwrap_or(false)
    }

    fn update(&mut self, timestamp: u32) {
        self.last_timestamp_ms = Some(timestamp);
        self.initialized = true;
    }
}

pub fn build_and_output_tables(
    parser: &can_decode::Parser,
    decoded_messages: impl Iterator<Item = logs::parse::LogMessage>,
    output_dir: PathBuf,
) {
    let header = TableHeader::new(parser);
    let num_cols = 1 + header.indexer.len();

    let mut output_file: Option<OutputFile> = None;
    let mut file_counter = 0;
    let mut timing = TimingState::new();
    let mut row: Option<RowBuilder> = None;

    for log_msg in decoded_messages {
        let timestamp_ms = log_msg.timestamp;

        // Check if we need to start a new file
        if timing.should_start_new_file(timestamp_ms) {
            // Create new file
            match OutputFile::new(&output_dir, file_counter, &header) {
                Ok(file) => {
                    output_file = Some(file);
                    file_counter += 1;
                }
                Err(e) => {
                    eprintln!("Failed to create output file: {}", e);
                    timing.update(timestamp_ms);
                    continue;
                }
            }

            // Initialize row builder
            row = Some(RowBuilder::new(num_cols, timestamp_ms as u64));
            timing.update(timestamp_ms);
        }

        let Some(ref mut file) = output_file else {
            eprintln!("No output file available; skipping message");
            timing.update(timestamp_ms);
            continue;
        };

        let Some(ref mut current_row) = row else {
            eprintln!("Row builder not initialized; skipping message");
            timing.update(timestamp_ms);
            continue;
        };

        // Flush rows until the message fits in the current time bin
        while current_row.should_advance(timestamp_ms as u64) {
            if let Err(e) = file.write_row(&current_row.data) {
                eprintln!("Failed to write row: {}", e);
            }
            current_row.clear_signals();
            current_row.advance_time();
        }

        // Update row with signal values
        for (sig_name, sig_val) in log_msg.decoded.signals {
            let key = (log_msg.decoded.name.clone(), sig_name.clone());
            if let Some(&col_idx) = header.indexer.get(&key) {
                let val_str = format!("{:?}", sig_val);
                current_row.set_signal(col_idx, val_str);
            } else {
                eprintln!(
                    "Warning: signal {:?} of message {:?} not found in header indexer",
                    sig_name, log_msg.decoded.name
                );
            }
        }

        timing.update(timestamp_ms);
    }

    // Write final row
    if let (Some(file), Some(row)) = (&mut output_file, &row) {
        if let Err(e) = file.write_row(&row.data) {
            eprintln!("Failed to write final row: {}", e);
        }
        if let Err(e) = file.writer.flush() {
            eprintln!("Failed to flush final output file: {}", e);
        }
    }
}
