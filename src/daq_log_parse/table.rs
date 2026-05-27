use crate::{
    daq_log_parse::{consts, correlate},
    util,
};

pub struct TableBuilder {
    bus_row: Vec<String>,
    node_row: Vec<String>,
    message_row: Vec<String>,
    message_desc_row: Vec<String>,
    signal_row: Vec<String>,
    signal_desc_row: Vec<String>,
    signal_unit_row: Vec<String>,

    // Key is (bus name, msg name, signal name), value is column index
    indexer: std::collections::HashMap<(String, String, String), usize>,
    next_col_idx: usize,
}

impl TableBuilder {
    pub fn new() -> Self {
        Self {
            bus_row: vec!["".to_string(), "".to_string(), "Bus".to_string()],
            node_row: vec!["".to_string(), "".to_string(), "Node".to_string()],
            message_row: vec!["".to_string(), "".to_string(), "Message".to_string()],
            message_desc_row: vec![
                "".to_string(),
                "".to_string(),
                "Message Description".to_string(),
            ],
            signal_row: vec![
                "Real Time".to_string(),
                "DAQ Timestamp".to_string(),
                "Signal".to_string(),
            ],
            signal_desc_row: vec![
                "".to_string(),
                "".to_string(),
                "Signal Description".to_string(),
            ],
            signal_unit_row: vec!["".to_string(), "".to_string(), "Signal Unit".to_string()],
            next_col_idx: 3, // real time, daq timestamp, then row headers columns
            indexer: std::collections::HashMap::new(),
        }
    }

    pub fn create_header(&mut self, parser: &can_decode::Parser, bus_name: &str) {
        let mut message_defs = parser.msg_defs();
        message_defs.sort_by_key(|m| util::can::can_dbc_to_u32_without_extid_flag(&m.id));

        for msg in message_defs {
            let bus_id = bus_name;
            let node = match msg.transmitter {
                can_dbc::Transmitter::NodeName(n) => n,
                can_dbc::Transmitter::VectorXXX => "N/A".to_string(),
            };

            for sig in msg.signals {
                let key = (bus_id.to_string(), msg.name.clone(), sig.name.clone());
                if let std::collections::hash_map::Entry::Vacant(e) = self.indexer.entry(key) {
                    e.insert(self.next_col_idx);

                    let msg_id_u32 = util::can::can_dbc_to_u32_with_extid_flag(&msg.id);

                    let msg_desc = parser
                        .msg_desc(msg_id_u32)
                        .map(|d| d.to_string())
                        .unwrap_or("".to_string());
                    let sig_desc = parser
                        .signal_desc(msg_id_u32, &sig.name)
                        .map(|d| d.to_string())
                        .unwrap_or("".to_string());

                    self.bus_row.push(bus_id.to_string());
                    self.node_row.push(node.to_string());
                    self.message_row.push(msg.name.to_string());
                    self.message_desc_row.push(msg_desc);
                    self.signal_row.push(sig.name.to_string());
                    self.signal_desc_row.push(sig_desc);
                    self.signal_unit_row.push(sig.unit.to_string());
                    self.next_col_idx += 1;
                }
            }
        }
    }

    pub fn create_and_write_tables(
        &self,
        out_folder: &std::path::Path,
        correlated_chunks: Vec<correlate::CorrelationChunkResult>,
    ) {
        std::fs::create_dir_all(out_folder).unwrap();

        for (chunk_idx, chunk) in correlated_chunks.iter().enumerate() {
            let mut csv_table = vec![
                self.bus_row.clone(),
                self.node_row.clone(),
                self.message_row.clone(),
                self.message_desc_row.clone(),
                self.signal_desc_row.clone(),
                self.signal_unit_row.clone(),
                self.signal_unit_row.clone(),
            ];

            let first_time = chunk.parsed_msgs.first().map(|m| m.timestamp).unwrap_or(0);
            let last_time = chunk.parsed_msgs.last().map(|m| m.timestamp).unwrap_or(0);

            let first_row_time = (first_time / consts::BIN_WIDTH_MS) * consts::BIN_WIDTH_MS;
            let last_row_time = last_time.div_ceil(consts::BIN_WIDTH_MS) * consts::BIN_WIDTH_MS;
            let num_rows = ((last_row_time - first_row_time) / consts::BIN_WIDTH_MS) + 1;

            csv_table.reserve(num_rows as usize);

            for row_idx in 0..num_rows {
                let row_time = first_row_time + row_idx * consts::BIN_WIDTH_MS;
                let row_time_sec = row_time as f32 / 1000.0;
                let mut row = vec!["".to_string(); self.bus_row.len()];
                let correlated_time = chunk
                    .correlation_fn
                    .as_ref()
                    .and_then(|cf| cf.correlate(row_time));
                if let Some(ct) = correlated_time {
                    row[0] = ct.format("%H:%M:%S.%3f").to_string();
                }
                row[1] = format!("{:.3}", row_time_sec);
                csv_table.push(row);
            }

            for msg in chunk.parsed_msgs.iter() {
                let decoded = &msg.decoded;
                for (sig_name, sig_value) in &decoded.signals {
                    let key = (msg.bus_name.clone(), decoded.name.clone(), sig_name.clone());
                    if let Some(&col_idx) = self.indexer.get(&key) {
                        let row_idx = (msg.timestamp - first_row_time) / consts::BIN_WIDTH_MS;
                        if let Some(row) = csv_table.get_mut(row_idx as usize + 4)
                            && let Some(cell) = row.get_mut(col_idx)
                        {
                            *cell = if let Some(enum_label) = &sig_value.value.enum_label {
                                format!("{} ({})", enum_label, sig_value.value.int_rounded())
                            } else {
                                sig_value.value.physical.to_string()
                            };
                        }
                    }
                }
            }

            let first_correlated_time: Option<String> =
                chunk.correlation_fn.as_ref().and_then(|cf| {
                    cf.correlate(first_time)
                        .map(|dt| dt.format("%Y_%m_%d__%H_%M_%S").to_string())
                });

            let out_file = match first_correlated_time {
                Some(t) => out_folder.join(format!("out_{:03}_{}.csv", chunk_idx, t)),
                None => out_folder.join(format!("out_{:03}.csv", chunk_idx)),
            };
            let mut wtr = csv::Writer::from_path(out_file.clone()).unwrap();
            for row in csv_table {
                wtr.write_record(&row).unwrap();
            }
            wtr.flush().unwrap();
            println!("Wrote chunk {} to CSV ({})", chunk_idx, out_file.display());
        }
    }
}
