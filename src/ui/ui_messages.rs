pub enum UiMessage {
    DbcSelected(std::path::PathBuf),
    SendCanMessage {
        msg_id: u32,
        msg_id_extended: bool,
        msg_bytes: Vec<u8>,
    },
}
