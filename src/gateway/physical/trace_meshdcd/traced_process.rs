use crate::tap::{Event, Timestamp};
use libc::{c_char, c_int, pid_t, waitpid, SIGTRAP, WIFSTOPPED, WSTOPSIG};
use libc::{
    siginfo_t, user_regs, ESRCH, PTRACE_ATTACH, PTRACE_DETACH, PTRACE_GETREGS,
    PTRACE_O_TRACESYSGOOD, PTRACE_SETOPTIONS, PTRACE_SYSCALL, __WALL,
};
use std::os::unix::fs::FileExt;
use std::ptr::null_mut;

pub type Result<T, E = TraceError> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct TracedProcess {
    pid: pid_t,
    fd: c_int,
    mem: std::fs::File,
}

#[derive(thiserror::Error, Debug)]
pub enum AttachError {
    #[error("PTRACE_ATTACH failed: {0}")]
    Ptrace(TraceError),
    #[error("error opening /proc/_/mem: {0}")]
    OpeningMem(std::io::Error),
    #[error("trace setup failed: {0}")]
    TraceSetup(TraceError),
}

impl TracedProcess {
    pub(crate) fn new(pid: pid_t, fd: c_int) -> Result<Self, AttachError> {
        assert_ne!(pid, 0);

        // Attach the process
        unsafe { ptrace(PTRACE_ATTACH, pid, null_mut(), 0) }.map_err(AttachError::Ptrace)?;

        // Open memory
        let mem = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(format!("/proc/{}/mem", pid));
        let mem = mem.map_err(|e| {
            // We're about to fail, but make sure we don't leave them hanging
            must_detach(pid);
            AttachError::OpeningMem(e)
        })?;

        // We can construct a TracedProcess, and we should since we want RAII
        let mut this = Self { pid, fd, mem };

        // Finish setup
        this.setup().map_err(AttachError::TraceSetup)?;

        Ok(this)
    }

    fn setup(&mut self) -> Result<()> {
        // Indicate that we want to identify system calls more easily
        unsafe {
            ptrace(
                PTRACE_SETOPTIONS,
                self.pid,
                null_mut(),
                PTRACE_O_TRACESYSGOOD,
            )?;
        }

        Ok(())
    }

    pub fn detach(mut self) {
        must_detach(self.pid);
        self.pid = 0;
    }

    fn wait_for_stop(&mut self) -> Result<c_int> {
        // Wait for the tracee to stop
        let mut status: c_int = 0;
        loop {
            let pid = unsafe { waitpid(self.pid, &mut status as *mut c_int, __WALL) };
            if pid > 0 && WIFSTOPPED(status) {
                return Ok(status);
            } else if pid < 0 {
                return Err(TraceError::errno());
            }
        }
    }

    fn wait_for_syscall_stop(&mut self) -> Result<()> {
        loop {
            let status = self.wait_for_stop()?;

            // Tracee is stopped
            // Is it stopped for a system call?
            if WSTOPSIG(status) == SIGTRAP | 0x80 {
                return Ok(());
            }

            // It stopped for some other reason
            // Resume
            self.continue_until_syscall()?;
        }
    }

    fn continue_until_syscall(&mut self) -> Result<(), TraceError> {
        unsafe { ptrace(PTRACE_SYSCALL, self.pid, null_mut(), 0) }
            .map_err(|e| {
                eprintln!("PTRACE_SYSCALL error");
                e
            })
            .map(|_| ())
    }

    fn wait_for_next_event(&mut self) -> Result<Event, TraceError> {
        loop {
            self.wait_for_syscall_stop()?;

            // We're stopped at a syscall
            // Get registers
            let regs = get_registers(self.pid)?;

            // Wait for the syscall to return
            self.continue_until_syscall()?;
            self.wait_for_stop()?;

            // Do we care about this syscall?
            // syscall # is r7, args are r0…r6
            let event = match regs.arm_r7 {
                SYSCALL_READ if regs.arm_r0 == self.fd as _ => {
                    // We're reading the FD to trace
                    // Wait for the system call to return
                    self.generate_read_event(regs)?
                }
                SYSCALL_WRITE if regs.arm_r0 == self.fd as _ => {
                    // We're writing the FD to trace
                    // Wait for the system call to return
                    self.generate_write_event(regs)?
                }
                _ => {
                    // Nah
                    None
                }
            };
            self.continue_until_syscall()?;

            if let Some(e) = event {
                return Ok(e);
            }

            // Go around again
        }
    }

    fn generate_read_event(&self, call_regs: user_regs) -> Result<Option<Event>> {
        let now = Timestamp::now();

        let return_regs = get_registers(self.pid)?;
        let buffer_ptr = call_regs.arm_r1;

        let bytes_read = return_regs.arm_r0 as isize;
        if bytes_read < 0 {
            // made no progress
            return Ok(None);
        }

        let mut buffer = vec![0u8; bytes_read as usize];
        self.mem
            .read_at(&mut buffer, buffer_ptr as _)
            .map_err(TraceError::MemoryReadError)?;
        Ok(Some(Event::SerialRx(now, buffer)))
    }

    fn generate_write_event(&self, call_regs: user_regs) -> Result<Option<Event>> {
        let now = Timestamp::now();

        let return_regs = get_registers(self.pid)?;
        let buffer_ptr = call_regs.arm_r1;

        let bytes = return_regs.arm_r0 as isize;
        if bytes < 0 {
            // made no progress
            return Ok(None);
        }

        let mut buffer = vec![0u8; bytes as usize];
        self.mem
            .read_at(&mut buffer, buffer_ptr as _)
            .map_err(TraceError::MemoryReadError)?;
        Ok(Some(Event::SerialTx(now, buffer)))
    }
}

impl Iterator for TracedProcess {
    type Item = super::Event;

    fn next(&mut self) -> Option<Self::Item> {
        match self.wait_for_next_event() {
            Ok(e) => Some(e),
            Err(e) => return Some(Event::Error(e)),
        }
    }
}

const SYSCALL_READ: u32 = 3;
const SYSCALL_WRITE: u32 = 4;

impl Drop for TracedProcess {
    fn drop(&mut self) {
        if self.pid != 0 {
            must_detach(self.pid);
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum TraceError {
    #[error("process terminated")]
    ProcessTerminated,
    #[error("ptrace error: {0}")]
    General(std::io::Error),
    #[error("memory read failed: {0}")]
    MemoryReadError(std::io::Error),
}

impl TraceError {
    fn errno() -> Self {
        let e = std::io::Error::last_os_error();
        match e.raw_os_error() {
            Some(ESRCH) => TraceError::ProcessTerminated,
            _ => TraceError::General(e),
        }
    }
}

unsafe fn ptrace(
    request: libc::c_uint,
    pid: pid_t,
    addr: *mut libc::c_char,
    data: libc::c_int,
) -> Result<c_int, TraceError> {
    let rv = libc::ptrace(request as _, pid, addr, data);
    if rv == -1 {
        Err(TraceError::errno())
    } else {
        Ok(rv)
    }
}

fn must_detach(pid: pid_t) {
    unsafe { ptrace(PTRACE_DETACH, pid, null_mut(), 0) }.expect("PTRACE_DETACH");
}

fn get_registers(pid: pid_t) -> Result<user_regs> {
    let mut regs: user_regs = unsafe { std::mem::zeroed() };
    unsafe { ptrace(PTRACE_GETREGS, pid, null_mut(), &mut regs as *mut _ as _) }.map_err(|e| {
        eprintln!("PTRACE_GETREGS error");
        e
    })?;
    Ok(regs)
}
