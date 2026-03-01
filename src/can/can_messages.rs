use crate::can;

pub enum CanMessage {
    ParsedMessage(can::message::ParsedMessage),
    Disconnection,
    ConnectionSuccessful,
    ConnectionFailed(String),
}
