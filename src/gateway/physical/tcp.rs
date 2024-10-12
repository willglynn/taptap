use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};

/// A TCP serial connection.
#[derive(Debug)]
pub struct Connection {
    socket: TcpStream,
    readonly: bool,
}

impl Connection {
    pub fn connect<A: ToSocketAddrs>(addr: A, readonly: bool) -> Result<Self, std::io::Error> {
        let socket = TcpStream::connect(addr)?;

        Ok(Self { socket, readonly })
    }
}

impl super::Connection for Connection {}

impl Read for Connection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.socket.read(buf)
    }
}

impl Write for Connection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.readonly {
            Err(std::io::ErrorKind::Unsupported.into())
        } else {
            self.socket.write(buf)
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if self.readonly {
            Ok(())
        } else {
            self.socket.flush()
        }
    }
}
