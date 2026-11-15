//! A loopback-only startup barrier for deterministic probe cleanup/reload tests.
//! The real node and product binaries never read this fake-only control.
use std::{
    io::{self, Read as _, Write as _},
    net::{SocketAddr, TcpStream},
    time::Duration,
};

const SAFETY: Duration = Duration::from_secs(30);

pub(super) fn wait() -> io::Result<()> {
    let Some(address) = std::env::var_os("FAKE_ACP_START_GATE") else {
        return Ok(());
    };
    let address: SocketAddr = address
        .to_string_lossy()
        .parse()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    if !address.ip().is_loopback() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "startup gate must be loopback",
        ));
    }
    let mut stream = TcpStream::connect_timeout(&address, SAFETY)?;
    stream.set_read_timeout(Some(SAFETY))?;
    stream.set_write_timeout(Some(SAFETY))?;
    stream.write_all(&std::process::id().to_be_bytes())?;
    let mut release = [0_u8; 1];
    stream.read_exact(&mut release)?;
    if release != [1] {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "startup gate refused release",
        ));
    }
    Ok(())
}
