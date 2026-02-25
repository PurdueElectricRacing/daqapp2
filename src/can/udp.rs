use slcan::CanFrame;
use std::{io, net::UdpSocket, time::Duration};

pub struct UdpDriver {
    _socket: UdpSocket,
}

impl UdpDriver {
    pub fn new(port: u16) -> io::Result<Self> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", port))?;
        socket.set_read_timeout(Some(Duration::from_millis(10)))?;
        Ok(Self { _socket: socket })
    }

    pub fn read_frame(&mut self) -> io::Result<CanFrame> {
        todo!("UDP Driver is not yet implemented")
    }
}
