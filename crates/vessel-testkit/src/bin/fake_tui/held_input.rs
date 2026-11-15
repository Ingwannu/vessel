//! Never starts an stdin reader before an independent loopback release command.
//! Raw/no-echo mode makes a full OS/control queue real, rather than canonical
//! terminal line truncation or echo output pretending to be writer pressure.
use std::io::{self, BufRead as _, Read as _, Write as _};
use std::net::{Ipv4Addr, TcpListener};
use std::process::ExitCode;

const MAX_DRAIN: u64 = 16 * 1024 * 1024;
const EXPIRED: &[u8] = b"NODE-REPLY-EXPIRED-CANARY";

pub(super) fn run() -> ExitCode {
    match held() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            super::say(&format!("held input fixture error: {error}"));
            ExitCode::FAILURE
        }
    }
}

fn held() -> io::Result<()> {
    let mut mode = rustix::termios::tcgetattr(io::stdin())?;
    mode.make_raw();
    rustix::termios::tcsetattr(io::stdin(), rustix::termios::OptionalActions::Now, &mode)?;
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    super::say(&format!("control {}", listener.local_addr()?.port()));
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
    io::stdin()
        .lock()
        .take(MAX_DRAIN)
        .read_until(b'\n', &mut input)?;
    if !input.ends_with(b"done\n") {
        return Err(io::Error::other("fixture drain terminator missing"));
    }
    let expired = input.windows(EXPIRED.len()).any(|part| part == EXPIRED);
    super::say(&format!("drained {} expired {expired}", input.len()));
    Ok(())
}
