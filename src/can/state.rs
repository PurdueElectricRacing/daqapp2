use crate::messages;

pub struct State {
    pub can_to_ui_tx: std::sync::mpsc::Sender<messages::MsgFromCan>,
    pub ui_to_can_rx: std::sync::mpsc::Receiver<messages::MsgFromUi>,
    pub is_connected: bool,
    pub parser: Option<can_decode::Parser>,
}

impl State {
    pub fn new(
        can_to_ui_tx: std::sync::mpsc::Sender<messages::MsgFromCan>,
        ui_to_can_rx: std::sync::mpsc::Receiver<messages::MsgFromUi>,
    ) -> Self {
        Self {
            can_to_ui_tx,
            ui_to_can_rx,
            is_connected: false,
            parser: None,
        }
    }
}
