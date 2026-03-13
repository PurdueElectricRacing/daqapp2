use crate::daq_log_parse::consts;
use crate::daq_log_parse::parse;
use can_decode::DecodedSignalValue;

pub struct TableBuilder {
    bus_row: Vec<String>,
    node_row: Vec<String>,
    message_row: Vec<String>,
    signal_row: Vec<String>,

    // Key is (msg name, signal name), value is column index
    indexer: std::collections::HashMap<(String, String), usize>,
}

impl TableBuilder {
    pub fn new() -> Self {
        Self {
            bus_row: Vec::new(),
            node_row: Vec::new(),
            message_row: Vec::new(),
            signal_row: Vec::new(),
            indexer: std::collections::HashMap::new(),
        }
    }

    pub fn create_header(&mut self, parser: &can_decode::Parser) {
        // old code from per log parser, idk why it doesnt work here
        let mut message_defs = parser.msg_defs();
        message_defs.sort_by_key(|m| match m.id {
            can_dbc::MessageId::Standard(id) => id as u32,
            can_dbc::MessageId::Extended(id) => id,
        });

        self.bus_row.push("Bus".to_string());
        self.node_row.push("Node".to_string());
        self.message_row.push("Message".to_string());
        self.signal_row.push("Signal".to_string());

        let mut col_idx = 1;

        for msg in message_defs {
            let bus_id = "Main";
            let node = match msg.transmitter {
                can_dbc::Transmitter::NodeName(n) => n,
                can_dbc::Transmitter::VectorXXX => "N/A".to_string(),
            };

            for sig in msg.signals {
                let key = (msg.name.clone(), sig.name.clone());
                if let std::collections::hash_map::Entry::Vacant(e) = self.indexer.entry(key) {
                    e.insert(col_idx);
                    col_idx += 1;

                    self.bus_row.push(bus_id.to_string());
                    self.node_row.push(node.to_string());
                    self.message_row.push(msg.name.to_string());
                    self.signal_row.push(sig.name.to_string());
                }
            }
        }
    }

    pub fn create_and_write_tables(
        &self,
        out_folder: &std::path::Path,
        chunked_parsed: Vec<Vec<parse::ParsedMessage>>,
    ) {
        std::fs::create_dir_all(out_folder).unwrap();

        for (chunk_idx, chunk) in chunked_parsed.iter().enumerate() {
            let mut csv_table = vec![
                self.bus_row.clone(),
                self.node_row.clone(),
                self.message_row.clone(),
                self.signal_row.clone(),
            ];

            let first_time = chunk.first().map(|m| m.timestamp).unwrap_or(0);
            let last_time = chunk.last().map(|m| m.timestamp).unwrap_or(0);

            let first_row_time = (first_time / consts::BIN_WIDTH_MS) * consts::BIN_WIDTH_MS;
            let last_row_time = last_time.div_ceil(consts::BIN_WIDTH_MS) * consts::BIN_WIDTH_MS;
            let num_rows = ((last_row_time - first_row_time) / consts::BIN_WIDTH_MS) + 1;

            csv_table.reserve(num_rows as usize);

            for row_idx in 0..num_rows {
                let row_time = first_row_time + row_idx * consts::BIN_WIDTH_MS;
                let row_time_sec = row_time as f32 / 1000.0;
                let mut row = vec!["".to_string(); self.bus_row.len()];
                row[0] = format!("{:.3}", row_time_sec);
                csv_table.push(row);
            }

            for msg in chunk {
                let decoded = &msg.decoded;
                for (sig_name, sig_value) in &decoded.signals {
                    let key = (decoded.name.clone(), sig_name.clone());
                    if let Some(&col_idx) = self.indexer.get(&key) {
                        let row_idx = (msg.timestamp - first_row_time) / consts::BIN_WIDTH_MS;
                        if let Some(row) = csv_table.get_mut(row_idx as usize + 4)
                            && let Some(cell) = row.get_mut(col_idx)
                        {
                            *cell = match &sig_value.value {
                                DecodedSignalValue::Numeric(v) => v.to_string(),
                                DecodedSignalValue::Enum(_, label) => label.clone(),
                            };
                        }
                    }
                }
            }
            let out_file = out_folder.join(format!("out_{:03}.csv", chunk_idx));
            let mut wtr = csv::Writer::from_path(out_file).unwrap();
            for row in csv_table {
                wtr.write_record(&row).unwrap();
            }
            wtr.flush().unwrap();
            println!("Wrote chunk {} to CSV", chunk_idx);
        }
    }
}
