use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ConnectionSource {
    Serial(String),
    Udp(u16),
}
