pub enum CanMessage {
    DecodedMessage {
        timestamp: u64,
        decoded: can_decode::DecodedMessage,
    },
}
