use crate::config::TcpKeepaliveConfig;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};

/// A TCP serial connection.
#[derive(Debug)]
pub struct Connection {
    socket: TcpStream,
    readonly: bool,
}

impl Connection {
    pub fn connect<A: ToSocketAddrs>(
        addr: A,
        readonly: bool,
        keepalive: TcpKeepaliveConfig,
    ) -> Result<Self, std::io::Error> {
        let socket = TcpStream::connect(addr)?;

        enable_keepalive(&socket, keepalive)?;

        Ok(Self {
            socket,
            readonly,
        })
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

/// Enable TCP keepalive and configure its parameters as supported by the platform.
fn enable_keepalive(socket: &TcpStream, cfg: TcpKeepaliveConfig) -> std::io::Result<()> {
    //use std::os::unix::prelude::AsRawFd;
    use socket2::{Socket, TcpKeepalive};

    let sock = Socket::from(socket.try_clone()?);

    let mut keepalive = TcpKeepalive::new();

    //if let Some(idle) = cfg.idle {
    //    keepalive = keepalive.with_time(idle);
    //}
    keepalive = keepalive.with_time(cfg.idle);
    //if let Some(interval) = cfg.interval {
    //    keepalive = keepalive.with_interval(interval);
    //}
    keepalive = keepalive.with_interval(cfg.interval);
    // Note: .with_retries() is not available on all platforms.
    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_os = "windows",
        target_os = "macos",
        target_os = "ios"
    ))]
    if true {
        keepalive = keepalive.with_retries(cfg.count);
    }
    //if let Some(count) = cfg.count {
    //    keepalive = keepalive.with_retries(count);
    //}

    sock.set_tcp_keepalive(&keepalive)?;

    Ok(())
}
