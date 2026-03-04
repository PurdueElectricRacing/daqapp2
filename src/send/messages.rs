
pub enum SendAmount {
	Infinite {
		period: usize,
	},
	Once,
	Finite {
		amount: usize,
		period: usize,
	}
}

pub struct AddMessage {
	pub amount: SendAmount,
	pub msg_id: u32,
	pub is_msg_id_extended: bool,
	pub msg_bytes: Vec<u8>,
}

pub enum ToSendThread {
	AddMessage(AddMessage),
	DeleteMessage {
		msg_id: u32,
	}
}

pub enum FromSendThread {
	MessageSent {
		msg_id: u32,
		timestamp: chrono::DateTime<chrono::Local>,
	},
}