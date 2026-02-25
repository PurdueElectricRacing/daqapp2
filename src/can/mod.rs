pub mod can_messages;
pub mod message;
pub mod serial;
pub mod thread;
pub mod udp;

use serde::{Deserialize, Serialize};
use slcan::CanFrame;
use std::io;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ConnectionSource {
    Serial(String),
    Udp(u16),
}

impl ConnectionSource {
    pub fn create_driver(&self) -> io::Result<Driver> {
        match self {
            Self::Serial(path) => Ok(Driver::Serial(serial::SerialDriver::new(
                path.clone(),
                115_200,
            )?)),
            Self::Udp(port) => Ok(Driver::Udp(udp::UdpDriver::new(*port)?)),
        }
    }
}

pub enum Driver {
    Serial(serial::SerialDriver),
    Udp(udp::UdpDriver),
}

impl Driver {
    pub fn read_frame(&mut self) -> io::Result<CanFrame> {
        match self {
            Self::Serial(s) => s.read_frame(),
            Self::Udp(u) => u.read_frame(),
        }
    }
}
