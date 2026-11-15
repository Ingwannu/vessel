//! Additive version-only fixture: proves execution and can hold that actual
//! process on its existing stdin. Never records argv, environment or credentials.

use std::io::{self, BufRead as _, Write as _};

pub(super) fn entered() -> io::Result<()> {
    if let Some(path) = std::env::var_os("FAKE_CODEX_VERSION_AUDIT") {
        let mut audit = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        audit.write_all(b"version\n")?;
    }
    if std::env::var_os("FAKE_CODEX_VERSION_WAIT_STDIN").is_some() {
        let mut release = String::new();
        if io::stdin().lock().read_line(&mut release)? == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "version fixture control closed",
            ));
        }
    }
    Ok(())
}
