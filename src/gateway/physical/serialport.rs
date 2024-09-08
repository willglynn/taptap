use serialport::Result;
use serialport::{
    available_ports, DataBits, FlowControl, Parity, SerialPort, SerialPortInfo, SerialPortType,
    StopBits,
};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct PortInfo(SerialPortInfo);

impl PortInfo {
    pub fn list() -> Result<Vec<PortInfo>> {
        available_ports().map(|vec| vec.into_iter().map(Self).collect())
    }

    pub fn open(&self) -> Result<Port> {
        Port::open(&self.0.port_name)
    }

    pub fn name(&self) -> &str {
        &self.0.port_name
    }

    pub fn port_type(&self) -> &SerialPortType {
        &self.0.port_type
    }
}

#[derive(Debug)]
pub struct Port {
    pub inner: Box<dyn SerialPort>,
}

impl Port {
    pub fn open(name: &str) -> Result<Self> {
        serialport::new(name, 38400)
            .data_bits(DataBits::Eight)
            .parity(Parity::None)
            .stop_bits(StopBits::One)
            .flow_control(FlowControl::None)
            .timeout(Duration::from_millis(5))
            .open()
            .map(Port::new)
    }

    fn new(inner: Box<dyn SerialPort>) -> Self {
        Port { inner }
    }
}

impl std::io::Read for Port {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            match self.inner.read(buf) {
                Ok(n) => return Ok(n),
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

impl std::io::Write for Port {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl super::Connection for Port {}
