use crate::connection::ConnectionSource;
use serde::de::value::Error;
use serialport::{ClearBuffer, SerialPort};
use slcan::sync::CanSocket;
use slcan::{CanFrame, NominalBitRate, OperatingMode};
use std::net::UdpSocket;
use std::time::Duration;

const SERIAL_BAUD_RATE: u32 = 115_200;
const SERIAL_TIMEOUT_MS: u64 = 10;

pub type DriverResult<T> = Result<T, DriverError>;

#[derive(Debug)]
pub enum DriverReadError {
    Timeout,
    IoError(String),
    Other(String),
}

#[derive(Debug)]
pub enum DriverError {
    ConnectionFailed(String),
    ReadError(DriverReadError),
    WriteError(String),
}

pub trait Driver {
    fn read_frame(&mut self) -> DriverResult<CanFrame>;

    fn write_frame(&mut self, frame: CanFrame) -> DriverResult<()>;

    fn is_connected(&self) -> bool;

    fn close(&mut self) -> DriverResult<()>;
}

/// Serial CAN driver using SLCAN protocol
pub struct SerialDriver {
    socket: CanSocket<Box<dyn SerialPort>>,
    connected: bool,
}

impl SerialDriver {
    pub fn new(port_path: &str) -> DriverResult<Self> {
        let port = serialport::new(port_path, SERIAL_BAUD_RATE)
            .timeout(Duration::from_millis(SERIAL_TIMEOUT_MS))
            .open()
            .map_err(|e| {
                DriverError::ConnectionFailed(format!("Failed to open port {}: {}", port_path, e))
            })?;

        let _ = port.clear(ClearBuffer::All);
        let mut socket = CanSocket::new(port);

        socket
            .set_operating_mode(OperatingMode::Normal)
            .map_err(|e| {
                DriverError::ConnectionFailed(format!("Failed to set operating mode: {}", e))
            })?;

        socket
            .open(NominalBitRate::Rate500Kbit)
            .map_err(|e| DriverError::ConnectionFailed(format!("Failed to open CAN: {}", e)))?;

        Ok(Self {
            socket,
            connected: true,
        })
    }
}

impl Driver for SerialDriver {
    fn read_frame(&mut self) -> DriverResult<CanFrame> {
        self.socket.read().map_err(|e| match e {
            slcan::ReadError::Io(io_err) => {
                if io_err.kind() == std::io::ErrorKind::WouldBlock
                    || io_err.kind() == std::io::ErrorKind::TimedOut
                {
                    DriverError::ReadError(DriverReadError::Timeout)
                } else {
                    self.connected = false;
                    DriverError::ReadError(DriverReadError::IoError(format!(
                        "I/O error: {}",
                        io_err
                    )))
                }
            }
            other => {
                self.connected = false;
                DriverError::ReadError(DriverReadError::Other(format!("Read error: {:?}", other)))
            }
        })
    }

    fn write_frame(&mut self, frame: CanFrame) -> DriverResult<()> {
        self.socket.send(frame).map_err(|e| {
            self.connected = false;
            DriverError::WriteError(format!("Failed to write frame: {}", e))
        })
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn close(&mut self) -> DriverResult<()> {
        self.socket
            .close()
            .map_err(|e| DriverError::WriteError(format!("Failed to close: {}", e)))?;
        self.connected = false;
        Ok(())
    }
}

/// UDP CAN driver (placeholder for future implementation)
pub struct UdpDriver {
    port: u16,
    socket: UdpSocket,
    connected: bool,
}

impl UdpDriver {
    /// Create a new UDP driver
    pub fn new(port: u16) -> DriverResult<Self> {
        // TODO: Implement UDP connection
        let udp_addr = format!("0.0.0.0:{}", port);
        let socket = UdpSocket::bind(udp_addr).map_err(|e| {
            DriverError::ConnectionFailed(format!("Failed to bind to port {}: {}", port, e))
        })?;
        // use a short read timeout instead of nonblocking so recv_from returns from timeout??
        // i think it should also work in nonblocking mode i just put it like this for testing
        socket.set_broadcast(true).map_err(|e| {
            DriverError::ConnectionFailed(format!("Failed to set broadcast: {}", e))
        })?;

        socket
            .set_read_timeout(Some(Duration::from_millis(5000)))
            .map_err(|e| {
                DriverError::ConnectionFailed(format!("Failed to set read timeout: {}", e))
            })?;
        log::info!("Socket local addr: {:?}", socket.local_addr());
        log::info!("Socket read timeout: {:?}", socket.read_timeout());

        Ok(Self {
            port,
            socket,
            connected: true,
        })
    }
}

impl Driver for UdpDriver {
    fn read_frame(&mut self) -> DriverResult<CanFrame> {
        // TODO: possibly need to handle the the fact that one UDP packet could contain multiple CAN frames

        log::info!("Trying to read UDP frame...");
        // and the data is in PER DAQ log format, not raw CAN frames
        let mut buf = [0; 2048];
        match self.socket.recv_from(&mut buf) {
            Ok((num_bytes, src_port)) => {
                // TODO: parse buffer into one or more CanFrame(s) per your protocol.
                log::info!("Recieved byte buf");
                parse_udp_buffer(&buf, num_bytes)
            }
            Err(e) => {
                log::warn!("{}", e);
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut
                {
                    std::thread::sleep(Duration::from_millis(10));
                    Err(DriverError::ReadError(DriverReadError::Timeout))
                } else {
                    self.connected = false;
                    Err(DriverError::ReadError(DriverReadError::IoError(format!(
                        "UDP I/O error: {}",
                        e
                    ))))
                }
            }
        }
    }

    fn write_frame(&mut self, _frame: CanFrame) -> DriverResult<()> {
        log::error!("UDP write requested but not implemented; ignoring frame.");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn close(&mut self) -> DriverResult<()> {
        self.connected = false;
        Ok(())
    }
}

pub fn parse_udp_buffer(buf: &[u8; 2048], num_bytes: usize) -> DriverResult<CanFrame> {
    if num_bytes < 5 {
        return Err(DriverError::ReadError(DriverReadError::Other(format!(
            " Received packet too small: {} bytes",
            num_bytes
        ))));
    }

    log::info!("Parsing UDP packet");
    // parse can frame from UDP packet according to new timestamped frame format
    // format: [4 bytes ticks_ms] [4 bytes identity] [8 bytes payload]
    // identity format: [1 bit bus ID] [1 bit isExtID] [1 bit reserved] [29 bits CAN ID]
    // (definitions from spmc.h)
    let identity = u32::from_le_bytes(buf[4..8].try_into().unwrap());
    let payload = &buf[8..16];
    let mask_id = (1u32 << 29) - 1;
    let id = identity & mask_id;
    if id <= 0x7FF {
        let sid = slcan::StandardId::new(id as u16).ok_or_else(|| {
            DriverError::ReadError(DriverReadError::Other("invalid standard id".into()))
        })?;
        let can2 = slcan::Can2Frame::new_data(sid, payload).ok_or_else(|| {
            DriverError::ReadError(DriverReadError::Other("invalid CAN2 data".into()))
        })?;
        Ok(can2.into())
    } else {
        //extid
        let eid = slcan::ExtendedId::new(id).ok_or_else(|| {
            DriverError::ReadError(DriverReadError::Other("invalid extended id".into()))
        })?;
        let can2 = slcan::Can2Frame::new_data(eid, payload).ok_or_else(|| {
            DriverError::ReadError(DriverReadError::Other("invalid CAN2 data".into()))
        })?;
        Ok(can2.into())
    }
}

pub fn create_driver(source: &ConnectionSource) -> DriverResult<Box<dyn Driver>> {
    match source {
        ConnectionSource::Serial(path) => Ok(Box::new(SerialDriver::new(path)?)),
        ConnectionSource::Udp(port) => Ok(Box::new(UdpDriver::new(*port)?)),
    }
}
