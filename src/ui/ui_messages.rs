use crate::connection;

pub enum UiMessage {
    DbcSelected(std::path::PathBuf),
    Connect(connection::ConnectionSource),
}
