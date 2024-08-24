//! The gateway physical layer.
//!
//! This layer is responsible for RS-485 communication with gateway(s) like Tigo TAPs. This crate
//! provides multiple implementations.
//!
//! * `serialport`, when compiled with the `serialport` feature
//! * [`tcp`]
//! * `termios`, when compiled on UNIX-like systems

use std::fmt::Debug;

pub trait Connection: std::io::Read + std::io::Write + Debug {}

pub mod serialport;

#[cfg(unix)]
pub mod termios;

pub mod tcp;

//#[cfg(all(target_arch = "armv7l", target_os = "linux"))]
//pub mod trace_meshdcd;
