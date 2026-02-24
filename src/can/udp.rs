use crate::can::CanDriver;
use slcan::CanFrame;
use std::{io, net::UdpSocket, thread, time::Duration};

pub struct UdpDriver {
    _socket: UdpSocket,
}

impl UdpDriver {
    pub fn new(port: u16) -> io::Result<Self> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", port))?;
        socket.set_read_timeout(Some(Duration::from_millis(10)))?;
        Ok(Self { _socket: socket })
    }
}

impl CanDriver for UdpDriver {
    fn read_frame(&mut self) -> io::Result<CanFrame> {
        // Mocking for now: simulate a delay and no data
        thread::sleep(Duration::from_millis(10));
        Err(io::Error::new(io::ErrorKind::WouldBlock, "Not data yet"))
    }

    fn write_frame(&mut self, _frame: &CanFrame) -> io::Result<()> {
        Ok(())
    }

    fn is_connected(&self) -> bool {
        true
    }
}
