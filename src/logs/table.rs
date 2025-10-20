use std::io::Write as _;

use crate::logs;

struct TableHeader {
    bus_row: Vec<String>,
    node_row: Vec<String>,
    message_row: Vec<String>,
    signal_row: Vec<String>,
    indexer: std::collections::HashMap<(String, String), usize>, // key is (msg, signal), value is column index
}

impl TableHeader {
    pub fn create(parser: &can_decode::Parser) -> Self {
        let mut message_defs = parser.msg_defs();
        message_defs.sort_by_key(|m| match m.message_id() {
            can_dbc::MessageId::Standard(id) => *id as u32,
            can_dbc::MessageId::Extended(id) => *id,
        });

        let mut bus_row = Vec::with_capacity(message_defs.len() + 1);
        let mut node_row = Vec::with_capacity(message_defs.len() + 1);
        let mut message_row = Vec::with_capacity(message_defs.len() + 1);
        let mut signal_row = Vec::with_capacity(message_defs.len() + 1);
        let mut indexer = std::collections::HashMap::new();

        bus_row.push("Bus".to_string());
        node_row.push("Node".to_string());
        message_row.push("Message".to_string());
        signal_row.push("Signal".to_string());

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
}

fn join_csv_row(cells: &[String]) -> String {
    // Minimal escaping: wrap cells containing comma or quote in double quotes and escape quotes
    let mut out = String::new();
    for (i, cell) in cells.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        if cell.contains(',') || cell.contains('"') || cell.contains('\n') {
            let escaped = cell.replace('"', "\"\"");
            out.push('"');
            out.push_str(&escaped);
            out.push('"');
        } else {
            out.push_str(cell);
        }
    }
    out.push('\n');
    out
}

pub fn build_and_output_tables(
    parser: &can_decode::Parser,
    decoded_messages: impl Iterator<Item = logs::parse::LogMessage>,
    output_dir: std::path::PathBuf,
) {
    // Create header (keeps only the header in memory besides one row)
    let header = TableHeader::create(parser);

    let header_bus_line = join_csv_row(&header.bus_row);
    let header_node_line = join_csv_row(&header.node_row);
    let header_message_line = join_csv_row(&header.message_row);
    let header_signal_line = join_csv_row(&header.signal_row);

    // Number of columns (first column reserved for time)
    let num_cols = 1 + header.indexer.len();

    // Row storage: Vec<Option<String>>; index 0 is time as string, rest are signal values
    let mut row: Vec<Option<String>> = vec![None; num_cols];

    // File management
    let mut out_file: Option<std::io::BufWriter<std::fs::File>> = None;
    let mut out_file_cnt: usize = 0;

    // Timing state (timestamps are assumed to be in milliseconds as u32)
    let mut start_t_valid = false;
    let mut last_t_ms: Option<u32> = None;
    let mut cur_row_t_ms: u64 = 0; // current row aligned time in ms

    // default behaviour: clear values after writing a row (like Python's fill_empty_vals=True)
    let fill_empty = true;

    // Helper: open a new output file and write header
    let new_out_file =
        |out_dir: &std::path::PathBuf,
         out_file_cnt: &mut usize,
         out_file_slot: &mut Option<std::io::BufWriter<std::fs::File>>| {
            if let Some(writer) = out_file_slot.take() {
                // flush and drop previous
                let _ = writer.into_inner().map(|mut f| f.flush());
            }
            let fname = out_dir.join(format!("out_{}.csv", out_file_cnt));
            *out_file_cnt += 1;
            let f = match std::fs::File::create(&fname) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Failed to create output file {:?}: {}", fname, e);
                    return;
                }
            };
            let mut writer = std::io::BufWriter::new(f);
            // write header lines
            if let Err(e) = writer.write_all(header_bus_line.as_bytes()) {
                eprintln!("Failed to write header to {:?}: {}", fname, e);
            }
            if let Err(e) = writer.write_all(header_node_line.as_bytes()) {
                eprintln!("Failed to write header to {:?}: {}", fname, e);
            }
            if let Err(e) = writer.write_all(header_message_line.as_bytes()) {
                eprintln!("Failed to write header to {:?}: {}", fname, e);
            }
            if let Err(e) = writer.write_all(header_signal_line.as_bytes()) {
                eprintln!("Failed to write header to {:?}: {}", fname, e);
            }
            *out_file_slot = Some(writer);
        };

    // Helper: write the current row to file
    let write_row_to_file = |writer_opt: &mut Option<std::io::BufWriter<std::fs::File>>,
                             row_vec: &Vec<Option<String>>| {
        if writer_opt.is_none() {
            return;
        }
        let writer = writer_opt.as_mut().unwrap();
        // convert row Vec<Option<String>> to Vec<String> where None -> ""
        let cells: Vec<String> = row_vec
            .iter()
            .map(|opt| match opt {
                Some(s) => s.clone(),
                None => "".to_string(),
            })
            .collect();
        let line = join_csv_row(&cells);
        if let Err(e) = writer.write_all(line.as_bytes()) {
            eprintln!("Failed to write row: {}", e);
        }
        // flush not required every row; BufWriter will buffer
    };

    // iterate incoming decoded messages
    for log_msg in decoded_messages {
        // Assumption: log_msg.timestamp is milliseconds (u32)
        let t_ms = log_msg.timestamp;

        // When message arrives, determine if we need to start a new output file
        if !start_t_valid
            || last_t_ms
                .map(|last| t_ms < last || t_ms.wrapping_sub(last) >= logs::consts::MAX_JUMP_MS)
                .unwrap_or(false)
        {
            // start a new file
            new_out_file(&output_dir, &mut out_file_cnt, &mut out_file);

            // Clear row values after starting new file
            for v in row.iter_mut() {
                *v = None;
            }

            start_t_valid = true;
            // align cur_row_t_ms to bin size floor
            cur_row_t_ms = (t_ms as u64 / logs::consts::BIN_WIDTH_MS as u64)
                * logs::consts::BIN_WIDTH_MS as u64;
            // set time column as seconds with 5 decimals
            let secs = (cur_row_t_ms as f64) / 1000.0;
            row[0] = Some(format!("{:.5}", secs));
        }

        // ensure out_file exists (in case new_out_file failed)
        if out_file.is_none() {
            eprintln!("No output file available; skipping message");
            last_t_ms = Some(t_ms);
            continue;
        }

        // If message falls beyond current bin, flush rows until message fits
        // while t_ms >= cur_row_t_ms + bin_width
        while (t_ms as u64) >= (cur_row_t_ms + logs::consts::BIN_WIDTH_MS as u64) {
            // write current row
            write_row_to_file(&mut out_file, &row);
            if fill_empty {
                // reset values except time
                for v in row.iter_mut().skip(1) {
                    *v = None;
                }
            }
            // advance cur_row_t_ms by bin width
            cur_row_t_ms += logs::consts::BIN_WIDTH_MS as u64;
            let secs = (cur_row_t_ms as f64) / 1000.0;
            row[0] = Some(format!("{:.5}", secs));
        }

        let decoded = log_msg.decoded;

        for (sig_name, sig_val) in decoded.signals {
            let key = (decoded.name.to_string(), sig_name.to_string());
            if let Some(&col) = header.indexer.get(&key) {
                // convert sig_val to string: prefer ToString, else Debug
                let val_str = format!("{:?}", sig_val);
                // insert into row at the column index
                if col < row.len() {
                    row[col] = Some(val_str);
                } else {
                    eprintln!(
                        "Column index {} out of range for row length {}",
                        col,
                        row.len()
                    );
                }
            } else {
                // signal not present in header/indexer -> ignore
                eprintln!(
                    "Warning: signal {:?} of message {:?} not found in header indexer",
                    sig_name, decoded.name
                );
            }
        }

        last_t_ms = Some(t_ms);
    }

    if out_file.is_some() {
        write_row_to_file(&mut out_file, &row);
        if let Some(mut w) = out_file.take()
            && let Err(e) = w.flush()
        {
            eprintln!("Failed to flush final output file: {}", e);
        }
    }
}
