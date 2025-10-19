use crate::can;
pub enum CanMessage {
    ParsedMessage(can::message::ParsedMessage),
}
