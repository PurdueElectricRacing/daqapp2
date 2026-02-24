use crate::can::message::ParsedMessage;

pub enum CanMessage {
    ParsedMessage(ParsedMessage),
    ConnectionFailed(String),
    ConnectionSuccessful,
}
