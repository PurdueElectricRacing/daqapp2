use crate::{can, ui};

pub struct State {
    pub can_to_ui_tx: std::sync::mpsc::Sender<can::can_messages::CanMessage>,
    pub ui_to_can_rx: std::sync::mpsc::Receiver<ui::ui_messages::UiMessage>,
    pub is_connected: bool,
    pub parser: Option<can_decode::Parser>,
}

impl State {
    pub fn new(
        can_to_ui_tx: std::sync::mpsc::Sender<can::can_messages::CanMessage>,
        ui_to_can_rx: std::sync::mpsc::Receiver<ui::ui_messages::UiMessage>,
    ) -> Self {
        Self {
            can_to_ui_tx,
            ui_to_can_rx,
            is_connected: false,
            parser: None,
        }
    }
}
