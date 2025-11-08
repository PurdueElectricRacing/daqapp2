#[derive(Clone)]
pub struct ParsedMessage {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub raw_bytes: Vec<u8>,
    pub decoded: can_decode::DecodedMessage,
    pub tx_node: String,
}
