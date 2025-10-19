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
