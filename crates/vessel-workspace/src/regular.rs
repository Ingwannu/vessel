//! Opening a file for reading only when it is a regular one, judged on the
//! opened handle.
//!
//! [Decision Log]
//! - Purpose: a file checked by its path (`metadata`, `is_file`) can be
//!   swapped for a FIFO or a link before it is opened. Opening a FIFO for
//!   reading waits for a writer, which held a blocking thread, and with it
//!   a removal or a setup, without end.
//! - Choice: on Unix, open with `O_NONBLOCK | O_NOFOLLOW` (the open returns
//!   at once for a FIFO, and refuses a link in the last place), then
//!   `fstat` the handle. `O_NONBLOCK` does not change reads from a regular
//!   file. The node's attachments and the language servers' documents are
//!   read the same way.
//! - Elsewhere: a plain open and the same check on the handle.

use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;

/// `path` opened for reading, or `None` when what is there is not a
/// regular file.
///
/// # Errors
/// The open's or the handle's metadata's; on Unix a link in the last place
/// is one (`O_NOFOLLOW`).
pub(crate) fn open(path: &Path) -> io::Result<Option<File>> {
    let mut options = OpenOptions::new();
    let _read = options.read(true);
    #[cfg(unix)]
    {
        use nix::fcntl::OFlag;
        use std::os::unix::fs::OpenOptionsExt as _;
        let _flags = options.custom_flags((OFlag::O_NONBLOCK | OFlag::O_NOFOLLOW).bits());
    }
    let file = options.open(path)?;
    Ok(file.metadata()?.is_file().then_some(file))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn a_fifo_or_a_link_is_not_opened_and_nothing_waits() {
        let temp = tempfile::tempdir().unwrap();
        let fifo = temp.path().join("fifo");
        nix::unistd::mkfifo(&fifo, nix::sys::stat::Mode::S_IRWXU).unwrap();
        assert!(open(&fifo).unwrap().is_none());
        let file = temp.path().join("file");
        std::fs::write(&file, "x").unwrap();
        assert!(open(&file).unwrap().is_some());
        let link = temp.path().join("link");
        std::os::unix::fs::symlink(&file, &link).unwrap();
        assert!(open(&link).is_err());
        assert!(open(temp.path()).unwrap().is_none());
    }
}
