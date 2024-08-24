use libc::{c_int, pid_t};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Target {
    pub(crate) meshdcd_pid: pid_t,
    pub(crate) meshdcd_tty_fd: c_int,
}

#[derive(thiserror::Error, Debug)]
pub enum FindTargetError {
    #[error("reading /proc directory: {0}")]
    ReadProcDir(std::io::Error),
    #[error("reading /proc/{0}/exe: {1}")]
    ReadPidExe(pid_t, std::io::Error),
    #[error("reading /proc/{0}/fd: {1}")]
    ReadPidFds(pid_t, std::io::Error),
    #[error("no `meshdcd` process found")]
    NoMeshdcdProcess,
    #[error("`meshdcd` has no open tty")]
    MeshdcdHasNoTty,
    #[error("`meshdcd` has multiple open ttys")]
    MeshdcdHasMultipleTtys,
}

type Result<T, E = FindTargetError> = std::result::Result<T, E>;

fn read_proc_pids() -> Result<impl Iterator<Item = Result<pid_t>>> {
    let readdir = fs::read_dir("/proc").map_err(FindTargetError::ReadProcDir)?;
    Ok(readdir.filter_map(|result| {
        result
            .map_err(FindTargetError::ReadProcDir)
            .map(|entry| {
                // See if we can parse this entry as a PID
                std::str::from_utf8(entry.file_name().as_encoded_bytes())
                    .ok()
                    .and_then(|filename| filename.parse::<pid_t>().ok())
            })
            .transpose()
    }))
}

fn is_meshdcd(pid: pid_t) -> bool {
    // Failures here might mean that the process exited while we're identifying it
    // Ignore them
    fs::read_link(format!("/proc/{}/exe", pid))
        .ok()
        .map(|path| {
            // Is this meshdcd?
            path.as_os_str().as_encoded_bytes().ends_with(b"/meshdcd")
        })
        .unwrap_or(false)
}

fn fds(pid: pid_t) -> Result<Vec<(c_int, PathBuf)>> {
    let fd_path = PathBuf::from(format!("/proc/{}/fd", pid));
    fs::read_dir(&fd_path)
        .map_err(|e| FindTargetError::ReadPidFds(pid, e))?
        .map(|result| {
            let entry = result.map_err(|e| FindTargetError::ReadPidFds(pid, e))?;

            let fd: c_int = std::str::from_utf8(entry.file_name().as_encoded_bytes())
                .expect("fds must be UTF-8")
                .parse()
                .expect("fds must be ints");

            let entry_path = fd_path.clone().join(entry.file_name());
            let points_to =
                fs::read_link(&entry_path).map_err(|e| FindTargetError::ReadPidFds(pid, e))?;
            Ok((fd, points_to))
        })
        .collect()
}

impl Target {
    pub fn find() -> Result<Self, FindTargetError> {
        // Find meshdcd
        let meshdcd_pid = read_proc_pids()?
            .filter_map(|pid| match pid {
                Ok(pid) if is_meshdcd(pid) => Some(Ok(pid)),
                Ok(_) => None,
                Err(e) => Some(Err(e)),
            })
            .next()
            .ok_or(FindTargetError::NoMeshdcdProcess)??;

        // Now that we have meshdcd, find which file descriptor points to a TTY
        let mut tty_fds = fds(meshdcd_pid)?
            .into_iter()
            .filter(|(fd, target)| {
                target
                    .as_os_str()
                    .as_encoded_bytes()
                    .starts_with(b"/dev/tty")
            })
            .map(|(fd, _)| fd);
        let meshdcd_tty_fd = tty_fds.next().ok_or(FindTargetError::MeshdcdHasNoTty)?;
        if tty_fds.next().is_some() {
            return Err(FindTargetError::MeshdcdHasMultipleTtys);
        }

        Ok(Target {
            meshdcd_pid,
            meshdcd_tty_fd,
        })
    }
}
