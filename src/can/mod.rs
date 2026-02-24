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
    pub fn create_driver(&self) -> io::Result<Box<dyn CanDriver>> {
        match self {
            Self::Serial(path) => Ok(Box::new(serial::SerialDriver::new(path.clone(), 115_200))),
            Self::Udp(port) => Ok(Box::new(udp::UdpDriver::new(*port)?)),
        }
    }
}

pub trait CanDriver: Send {
    fn read_frame(&mut self) -> io::Result<CanFrame>;
    fn write_frame(&mut self, frame: &CanFrame) -> io::Result<()>;
    fn is_connected(&self) -> bool;
}
