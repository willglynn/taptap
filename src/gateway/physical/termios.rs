use libc::{
    cfsetspeed, tcgetattr, tcsetattr, termios, B38400, CLOCAL, CREAD, CRTSCTS, CS8, CSIZE, CSTOPB,
    ECHO, ISIG, IXANY, IXOFF, IXON, OCRNL, ONLCR, OPOST, PARENB, TCSANOW, VMIN, VTIME,
};
use std::io::Error;
use std::os::unix::io::AsRawFd;
use std::path::Path;

/// An open serial port.
#[derive(Debug)]
pub struct Port {
    file: std::fs::File,
}

impl Port {
    pub fn open<P: AsRef<Path>>(device: P) -> Result<Self, Error> {
        // Open the path which hopefully points to a serial port
        let file = std::fs::File::options()
            .read(true)
            .write(true)
            .open(device)?;

        unsafe {
            let fd = file.as_raw_fd();
            let mut tty: termios = std::mem::zeroed();

            // Get the terminal settings
            if tcgetattr(fd, &mut tty as *mut _) != 0 {
                return Err(Error::last_os_error());
            }

            // Use the helper ot set 38400 baud
            if cfsetspeed(&mut tty as *mut _, B38400) != 0 {
                return Err(Error::last_os_error());
            }

            // Now, in the structure directly, set:
            tty.c_cflag = (tty.c_cflag & !CSIZE) | CS8; // 8
            tty.c_cflag &= !PARENB; // N
            tty.c_cflag &= !CSTOPB; // 1

            tty.c_cflag &= !CRTSCTS; // no hardware flow control
            tty.c_iflag &= !(IXON | IXOFF | IXANY); // no software flow control
            tty.c_cflag |= CLOCAL; // disable modem status lines
            tty.c_cflag |= CREAD; // enable receiving
            tty.c_lflag &= !ECHO; // no local echo
            tty.c_lflag &= !ISIG; // don't interpret signal characters
            tty.c_oflag &= !OPOST; // don't post-process the output
            tty.c_oflag &= !(ONLCR | OCRNL); // specifically don't mangle CR/LF

            tty.c_cc[VMIN] = 1; // read at least 1 byte
            tty.c_cc[VTIME] = 0; // wait any amount of time for that byte

            // Update the FD
            if tcsetattr(fd, TCSANOW, &tty as *const _) != 0 {
                return Err(Error::last_os_error());
            }
        }

        Ok(Self { file })
    }
}

impl std::io::Read for Port {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl std::io::Write for Port {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}
