use crate::can::message::ParsedMessage;

pub enum CanMessage {
    ParsedMessage(ParsedMessage),
    ConnectionFailed { source: String, error: String },
    ConnectionSuccessful,
}
