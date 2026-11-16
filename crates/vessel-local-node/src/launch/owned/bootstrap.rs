//! A private name is correlation only; fresh stdout echo qualifies this handle.

use tokio::io::AsyncWriteExt as _;
use vessel_client::{LocalConnectError, LocalConnection, NodeClient};
use vessel_proto::local_launch::{
    BootstrapChallenge, BootstrapPipe, BootstrapReady, CHALLENGE_BYTES, Ready,
};

use super::{LaunchError, Owned};

const OPEN_PAUSE: std::time::Duration = std::time::Duration::from_millis(20);

fn entropy() -> Result<[u8; CHALLENGE_BYTES], LaunchError> {
    let mut bytes = [0; CHALLENGE_BYTES];
    // Reuse the OS RNG; IDs, clocks, counters or a PRNG are not launch entropy.
    getrandom::fill(&mut bytes).map_err(LaunchError::Entropy)?;
    Ok(bytes)
}

pub(super) fn pipe() -> Result<BootstrapPipe, LaunchError> {
    Ok(BootstrapPipe::from_bytes(entropy()?))
}

pub(super) async fn connect(
    pipe: BootstrapPipe,
    mut stdin: tokio::process::ChildStdin,
) -> Result<(LocalConnection, BootstrapChallenge), LaunchError> {
    stdin
        .write_all(pipe.encode().as_bytes())
        .await
        .map_err(LaunchError::Write)?;
    drop(stdin);
    let mut connection = loop {
        match NodeClient::open_local(&pipe.path()).await {
            Ok(connection) => break connection,
            Err(LocalConnectError::Open(source))
                if source.kind() == std::io::ErrorKind::NotFound =>
            {
                tokio::time::sleep(OPEN_PAUSE).await;
            }
            Err(error) => return Err(LaunchError::Bootstrap(error)),
        }
    };
    // Generate AFTER open, for this connection; never reuse the name seed.
    let challenge = BootstrapChallenge::from_bytes(entropy()?);
    connection
        .send_launch_challenge(challenge)
        .await
        .map_err(LaunchError::Bootstrap)?;
    Ok((connection, challenge))
}

pub(super) fn verify(
    owned: &Owned,
    line: &str,
    challenge: &BootstrapChallenge,
) -> Result<Ready, LaunchError> {
    use subtle::ConstantTimeEq as _;
    let ready = BootstrapReady::decode(line).map_err(LaunchError::Transcript)?;
    // Reuse subtle, rather than a hand-written comparison of public transcript
    // data whose match gates credential output on this same retained pipe.
    let matches = bool::from(ready.challenge().as_bytes().ct_eq(challenge.as_bytes()));
    if !owned.child.lifetime().is_live() || !matches {
        return Err(LaunchError::PeerUnverified);
    }
    Ok(Ready::new(ready.node()))
}
