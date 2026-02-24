pub enum ConnectionSource {
    Serial(String),
    Udp(u16),
}

pub enum UiMessage {
    DbcSelected(std::path::PathBuf),
    Connect(ConnectionSource),
    Disconnect,
}
