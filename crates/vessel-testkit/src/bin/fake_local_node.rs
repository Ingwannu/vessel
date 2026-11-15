//! A child-lifetime fixture, never shipped or selected by the product binary.
//!
//! The parent authors readiness through the real protocol encoder (or malformed
//! bytes for rejection), so no marker is copied here. A private control socket
//! gives an EOF event proving cleanup, without PID polling or sleep coordination.

#[cfg(unix)]
use std::io::Read as _;
use std::io::Write as _;
use std::process::ExitCode;

#[cfg(unix)]
const MAX_FIXTURE_BYTES: u64 = 64 * 1024;

#[cfg(unix)]
fn run() -> std::io::Result<()> {
    let control_path = std::env::var_os("FAKE_LOCAL_CONTROL_SOCKET")
        .ok_or_else(|| std::io::Error::other("fixture control socket missing"))?;
    let mut control = std::os::unix::net::UnixStream::connect(control_path)?;
    control.write_all(b"started\n")?;
    control.flush()?;
    if let Some(path) = std::env::var_os("FAKE_LOCAL_READY_FILE") {
        let file = std::fs::File::open(path)?;
        let mut bytes = Vec::new();
        let _read = file
            .take(MAX_FIXTURE_BYTES.saturating_add(1))
            .read_to_end(&mut bytes)?;
        if u64::try_from(bytes.len()).map_err(std::io::Error::other)? > MAX_FIXTURE_BYTES {
            return Err(std::io::Error::other("fixture readiness too large"));
        }
        let mut stdout = std::io::stdout().lock();
        stdout.write_all(&bytes)?;
        stdout.flush()?;
    }
    loop {
        std::thread::park();
    }
}

#[cfg(not(unix))]
fn run() -> std::io::Result<()> {
    Err(std::io::Error::other(
        "local launch control fixture requires Unix",
    ))
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _written = writeln!(std::io::stderr().lock(), "local launch fixture: {error}");
            ExitCode::FAILURE
        }
    }
}
