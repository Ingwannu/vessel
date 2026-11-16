//! A child-lifetime fixture, never shipped or selected by the product binary.
//!
//! The parent authors readiness through the real protocol encoder (or malformed
//! bytes for rejection), so no marker is copied here. A private control socket
//! gives an EOF event proving cleanup, without PID polling or sleep coordination.

use std::io::Read as _;
use std::io::Write as _;
use std::process::ExitCode;

const MAX_FIXTURE_BYTES: u64 = 64 * 1024;

fn synthetic(mut control: impl std::io::Read + std::io::Write) -> std::io::Result<()> {
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
    if std::env::var_os("FAKE_LOCAL_LIFETIME_FIXTURE").is_some() {
        let mut stdout = std::io::stdout().lock();
        stdout.write_all(b"lifetime ready\n")?;
        stdout.flush()?;
    }
    // Parent-owned IPC is also cleanup: EOF or x ends this synthetic child.
    // A probe after launcher exit proves actual survival without PID polling.
    let mut byte = [0_u8; 1];
    loop {
        match control.read_exact(&mut byte) {
            Ok(()) if byte == [b'x'] => return Ok(()),
            Ok(()) => {
                control.write_all(b"!")?;
                control.flush()?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(error) => return Err(error),
        }
    }
}

fn connected(control: impl std::io::Read + std::io::Write) -> std::io::Result<()> {
    #[cfg(unix)]
    if std::env::var_os("FAKE_LOCAL_IGNORE_TERMINATE").is_some() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let _entered = runtime.enter();
        // Install before the started ACK without running blocking std I/O on
        // an async worker. Notifications are deliberately never consumed.
        let _ignored = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        return synthetic(control);
    }
    synthetic(control)
}

fn run() -> std::io::Result<()> {
    if let Some(address) = std::env::var_os("FAKE_LOCAL_CONTROL_ADDRESS") {
        let address: std::net::SocketAddr = address
            .to_str()
            .ok_or_else(|| std::io::Error::other("fixture address is not UTF-8"))?
            .parse()
            .map_err(std::io::Error::other)?;
        if !address.ip().is_loopback() {
            return Err(std::io::Error::other("fixture address must be loopback"));
        }
        return connected(std::net::TcpStream::connect(address)?);
    }
    #[cfg(unix)]
    {
        let path = std::env::var_os("FAKE_LOCAL_CONTROL_SOCKET")
            .ok_or_else(|| std::io::Error::other("fixture control socket missing"))?;
        connected(std::os::unix::net::UnixStream::connect(path)?)
    }
    #[cfg(not(unix))]
    {
        Err(std::io::Error::other("fixture control address missing"))
    }
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
