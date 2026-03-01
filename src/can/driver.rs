use crate::connection::ConnectionSource;
use serialport::{ClearBuffer, SerialPort};
use slcan::sync::CanSocket;
use slcan::{CanFrame, NominalBitRate, OperatingMode};
use std::time::Duration;

const SERIAL_BAUD_RATE: u32 = 115_200;
const SERIAL_TIMEOUT_MS: u64 = 10;

pub type DriverResult<T> = Result<T, DriverError>;

#[derive(Debug)]
pub enum DriverError {
    ConnectionFailed(String),
    ReadError(String),
    WriteError(String),
}

pub trait Driver {
    fn read_frame(&mut self) -> DriverResult<CanFrame>;

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
        let mut socket = CanSocket::new(port.try_clone().expect("Failed to clone serial port"));

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
                    DriverError::ReadError("Timeout".to_string())
                } else {
                    self.connected = false;
                    DriverError::ReadError(format!("IO error: {}", io_err))
                }
            }
            other => {
                self.connected = false;
                DriverError::ReadError(format!("Read error: {}", other))
            }
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
    connected: bool,
}

impl UdpDriver {
    /// Create a new UDP driver
    pub fn new(port: u16) -> DriverResult<Self> {
        // TODO: Implement UDP connection
        log::warn!("UDP driver not yet implemented");
        Err(DriverError::ConnectionFailed(
            "UDP not implemented".to_string(),
        ))
    }
}

impl Driver for UdpDriver {
    fn read_frame(&mut self) -> DriverResult<CanFrame> {
        // TODO: possibly need to handle the the fact that one UDP packet could contain multiple CAN frames
        // and the data is in PER DAQ log format, not raw CAN frames
        Err(DriverError::ReadError("UDP not implemented".to_string()))
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn close(&mut self) -> DriverResult<()> {
        self.connected = false;
        Ok(())
    }
}

pub fn create_driver(source: &ConnectionSource) -> DriverResult<Box<dyn Driver>> {
    match source {
        ConnectionSource::Serial(path) => Ok(Box::new(SerialDriver::new(path)?)),
        ConnectionSource::Udp(port) => Ok(Box::new(UdpDriver::new(*port)?)),
    }
}
