//! A module to get input by using `ptrace` on `meshdcd`.
//!
//! `ptrace()` is a general purpose Linux process tracing mechanism. Some Tigo owners have `root`
//! access on their controller devices, in which case they are free to use `ptrace()` to trace the
//! `meshdcd` process which interfaces with the local serial port.
//!
//! This module inspects the `/proc` filesystem to identify `meshdcd` and the file descriptor of the
//! local serial port. It then uses `ptrace()` to attach and intercept system calls. When `meshdcd`
//! `read()` or `write()`s the serial port, this module reads the buffer containing the serial data.

use std::error::Error;

mod target;
mod traced_process;

use target::Target;
use traced_process::TracedProcess;

#[derive(thiserror::Error, Debug)]
enum OpenError {
    #[error("error finding target process: {0}")]
    FindingTarget(#[from] target::FindTargetError),
    #[error("error attaching to `meshdcd` target: {0}")]
    Attaching(#[from] traced_process::AttachError),
}

pub fn open() -> Result<impl Iterator<Item = Blob>, impl Error> {
    let target = Target::find().map_err(OpenError::from)?;
    let attached =
        TracedProcess::new(target.meshdcd_pid, target.meshdcd_tty_fd).map_err(OpenError::from)?;
    Ok(attached)
}
