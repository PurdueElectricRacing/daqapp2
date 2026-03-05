use crate::connection;

pub enum MsgFromUi {
    DbcSelected(std::path::PathBuf),
    Connect(connection::ConnectionSource),
    AddSendMessage(AddSendMessage),
    DeleteSendMessage { msg_id: u32 },
}

pub enum MsgFromCan {
    ParsedMessage(ParsedMessage),
    Disconnection,
    ConnectionSuccessful,
    ConnectionFailed(String),
    MessageSent {
        msg_id: u32,
        timestamp: chrono::DateTime<chrono::Local>,
        amount_left: SendAmount,
    },
}

pub enum SendAmount {
    Infinite { period: usize },
    Once,
    Finite { amount: usize, period: usize },
}

impl SendAmount {
    pub fn subtract_one(&self) -> Option<Self> {
        match self {
            SendAmount::Infinite { period } => Some(SendAmount::Infinite { period: *period }),
            SendAmount::Once => None,
            SendAmount::Finite { amount, period } => {
                if *amount > 1 {
                    Some(SendAmount::Finite {
                        amount: *amount - 1,
                        period: *period,
                    })
                } else {
                    None
                }
            }
        }
    }
}

pub struct AddSendMessage {
    pub amount: SendAmount,
    pub msg_id: u32,
    pub is_msg_id_extended: bool,
    pub msg_bytes: Vec<u8>,
}

#[derive(Clone)]
pub struct ParsedMessage {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub raw_bytes: Vec<u8>,
    pub decoded: can_decode::DecodedMessage,
}
