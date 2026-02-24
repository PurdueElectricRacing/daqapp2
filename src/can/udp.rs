use crate::can::CanDriver;
use slcan::CanFrame;
use std::io;
use std::time::Duration;

pub struct UdpDriver {
    _socket: std::net::UdpSocket,
}

impl UdpDriver {
    pub fn new(port: u16) -> io::Result<Self> {
        let socket = std::net::UdpSocket::bind(format!("0.0.0.0:{}", port))?;
        socket.set_read_timeout(Some(Duration::from_millis(10)))?;
        Ok(Self { _socket: socket })
    }
}

impl CanDriver for UdpDriver {
    fn read_frame(&mut self) -> io::Result<CanFrame> {
        // Mocking for now: simulate a delay and no data
        std::thread::sleep(Duration::from_millis(10));
        Err(io::Error::new(io::ErrorKind::WouldBlock, "Not data yet"))
    }

    fn write_frame(&mut self, _frame: &CanFrame) -> io::Result<()> {
        Ok(())
    }

    fn is_connected(&self) -> bool {
        true
    }
}
