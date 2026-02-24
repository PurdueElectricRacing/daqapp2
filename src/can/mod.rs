pub mod can_messages;
pub mod message;
pub mod thread;
pub mod serial;
pub mod udp;

use slcan::CanFrame;
use std::io;

pub trait CanDriver: Send {
    fn read_frame(&mut self) -> io::Result<CanFrame>;
    fn write_frame(&mut self, frame: &CanFrame) -> io::Result<()>;
    fn is_connected(&self) -> bool;
}
