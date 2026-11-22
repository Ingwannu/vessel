//! A loopback-only load acknowledgement held by a test, with no timing races.
use std::io;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

pub(super) async fn wait(session: &str) -> io::Result<()> {
    wait_for("FAKE_ACP_LOAD_GATE", session).await
}

pub(super) async fn prompt(session: &str) -> io::Result<()> {
    wait_for("FAKE_ACP_PROMPT_GATE", session).await
}

async fn wait_for(variable: &str, session: &str) -> io::Result<()> {
    let Some(address) = std::env::var_os(variable) else {
        return Ok(());
    };
    let address: SocketAddr = address
        .to_str()
        .and_then(|text| text.parse().ok())
        .filter(|address: &SocketAddr| address.ip().is_loopback())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "the fake load gate must be a loopback socket",
            )
        })?;
    if session.len() > vessel_testkit::LOAD_GATE_SESSION_MAX_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "the fake load gate session id exceeds {} bytes",
                vessel_testkit::LOAD_GATE_SESSION_MAX_BYTES
            ),
        ));
    }
    let length = u32::try_from(session.len()).map_err(io::Error::other)?;
    let mut stream = TcpStream::connect(address).await?;
    stream.write_all(&length.to_be_bytes()).await?;
    stream.write_all(session.as_bytes()).await?;
    let mut answer = [0_u8; 1];
    let _read = stream.read_exact(&mut answer).await?;
    if answer != [1] {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "the fake load gate refused its acknowledgement",
        ));
    }
    Ok(())
}
