#[derive(Clone)]
pub struct ParsedMessage {
    // Timestamp in microseconds since epoch
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub decoded: can_decode::DecodedMessage,
}
