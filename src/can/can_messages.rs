use crate::can::message::ParsedMessage;
use std::path::PathBuf;

pub enum CanMessage {
    ParsedMessage(ParsedMessage),
    ConnectionFailed { source: String, error: String },
    ConnectionSuccessful,
}

pub enum WorkerCommand {
    Connect {
        source: crate::can::ConnectionSource,
        dbc_path: Option<PathBuf>,
    },
    Disconnect,
    UpdateDbc(Option<PathBuf>),
    Shutdown,
}
