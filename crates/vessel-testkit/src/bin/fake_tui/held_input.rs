//! Never starts an stdin reader before an independent loopback release command.
//! Raw/no-echo mode makes a full OS/control queue real, rather than canonical
//! terminal line truncation or echo output pretending to be writer pressure.
use std::io::{self, BufRead as _, Read as _, Write as _};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::process::ExitCode;
use std::time::Duration;

const MAX_DRAIN: u64 = 16 * 1024 * 1024;
const EXPIRED: &[u8] = b"NODE-REPLY-EXPIRED-CANARY";

pub(super) fn run(ready_port: Option<&str>) -> ExitCode {
    match held(ready_port) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            super::say(&format!("held input fixture error: {error}"));
            ExitCode::FAILURE
        }
    }
}

fn held(ready_port: Option<&str>) -> io::Result<()> {
    let mut mode = rustix::termios::tcgetattr(io::stdin())?;
    mode.make_raw();
    rustix::termios::tcsetattr(io::stdin(), rustix::termios::OptionalActions::Now, &mode)?;
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let port = listener.local_addr()?.port();
    super::say(&format!("control {port}"));
    if let Some(ready_port) = ready_port {
        signal_output_ready(ready_port, port)?;
    }
    let mut ticks = 0_u64;
    for stream in listener.incoming() {
        let mut stream = stream?;
        let mut command = [0_u8; 1];
        stream.read_exact(&mut command)?;
        match command {
            [b't'] => {
                ticks = ticks
                    .checked_add(1)
                    .ok_or_else(|| io::Error::other("fixture tick overflow"))?;
                super::say(&format!("held tick {ticks}"));
                stream.write_all(b"ok")?;
            }
            [b'r'] => break,
            _ => return Err(io::Error::other("invalid fixture control command")),
        }
    }
    // This is the first stdin read. A cancelled queue send must not reappear
    // when the real writer finally regains capacity.
    let mut input = Vec::new();
    let read = io::stdin()
        .lock()
        .take(MAX_DRAIN)
        .read_until(b'\n', &mut input)?;
    if read == 0 || !input.ends_with(b"done\n") {
        return Err(io::Error::other("fixture drain terminator missing"));
    }
    let expired = input.windows(EXPIRED.len()).any(|part| part == EXPIRED);
    super::say(&format!("drained {} expired {expired}", input.len()));
    Ok(())
}

/// A bounded banner larger than the PTY kernel buffer forces real output into
/// the existing reader channel before the separate readiness acknowledgement.
fn signal_output_ready(ready_port: &str, control_port: u16) -> io::Result<()> {
    let ready_port = ready_port
        .parse::<u16>()
        .map_err(|_invalid| io::Error::other("invalid fixture ready port"))?;
    {
        let mut output = io::stdout().lock();
        output.write_all(&vec![b'x'; 128 * 1024])?;
        output.write_all(b"\r\n")?;
        output.flush()?;
    }
    let mut ready = TcpStream::connect_timeout(
        &SocketAddrV4::new(Ipv4Addr::LOCALHOST, ready_port).into(),
        Duration::from_secs(5),
    )?;
    ready.set_write_timeout(Some(Duration::from_secs(5)))?;
    ready.write_all(&control_port.to_be_bytes())
}
