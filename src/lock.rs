//! A single-instance lock.
//!
//! M0 found that `docs/03-surface.md` was wrong about the MIDI port being
//! exclusive: on macOS `CoreMIDI` multiplexes, two processes drove the device
//! at once, and **neither was told**. So there is no `EBUSY` to catch and
//! no error to report — two `tallyd`s would simply repaint over each other
//! every tick, forever.
//!
//! This is a mutex with a path. Nothing is read from it and nothing is
//! written to it, so it is not the control socket
//! `docs/08-daemon.md` refuses.

use std::fs::File;
use std::io;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

/// Held for the life of the process. Dropping it (or exiting, however
/// abruptly) releases the lock — `flock` is tied to the file descriptor, so
/// even `SIGKILL` cannot leave a stale lock behind.
#[derive(Debug)]
pub struct Lock {
    _file: File,
    path: PathBuf,
}

impl Lock {
    /// Take the lock, or report who already has it.
    ///
    /// # Errors
    /// [`io::ErrorKind::WouldBlock`] if another process holds it; anything
    /// else means the lock file itself could not be opened.
    pub fn acquire(path: &Path) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::create(path)?;
        // SAFETY: `fd` is owned by `file`, which outlives this call, and
        // `flock` only ever inspects it. There is no safe wrapper for this
        // in std, and the alternative -- a lock crate -- buys one syscall.
        #[expect(unsafe_code, reason = "flock has no safe equivalent in std")]
        let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if rc != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            _file: file,
            path: path.to_owned(),
        })
    }

    /// Where the lock is held.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The conventional location: `$XDG_RUNTIME_DIR/tally.lock`, falling
    /// back to the temp dir, which is where macOS leaves us.
    pub fn default_path() -> PathBuf {
        crate::env_path("XDG_RUNTIME_DIR")
            .unwrap_or_else(std::env::temp_dir)
            .join("tally.lock")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_acquire_is_refused_while_the_first_lives() {
        let dir = std::env::temp_dir().join("tally-lock-test");
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("one.lock");

        let first = Lock::acquire(&path).expect("first acquire");
        // Same process, same fd table: flock is per-open-file-description,
        // so a second `File::create` is genuinely a second contender.
        let second = Lock::acquire(&path);
        assert!(second.is_err(), "a second instance must not get the lock");

        drop(first);
        let third = Lock::acquire(&path);
        assert!(third.is_ok(), "the lock must be released when dropped");
    }

    #[test]
    fn default_path_is_named_for_us() {
        assert!(Lock::default_path().ends_with("tally.lock"));
    }
}
