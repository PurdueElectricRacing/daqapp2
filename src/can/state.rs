use crate::{can, ui};

pub struct State {
	pub can_sender: std::sync::mpsc::Sender<can::can_messages::CanMessage>,
    pub ui_receiver: std::sync::mpsc::Receiver<ui::ui_messages::UiMessage>,
	pub is_connected: bool,
	pub parser: Option<can_decode::Parser>,
}

impl State {
	pub fn new(
		can_sender: std::sync::mpsc::Sender<can::can_messages::CanMessage>,
		ui_receiver: std::sync::mpsc::Receiver<ui::ui_messages::UiMessage>,
	) -> Self {
		Self {
			can_sender,
			ui_receiver,
			is_connected: false,
			parser: None,
		}
	}
}