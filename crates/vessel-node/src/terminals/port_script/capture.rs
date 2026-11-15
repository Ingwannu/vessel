//! Bounded foreign-process output is a decoder edge, never a log or a shell.

use tokio::io::{AsyncRead, AsyncReadExt as _};
use vessel_proc::{Spawned, Stream};
use vessel_proto::workspace::{ServicePort, trim_project_text};

use super::{MAX_OUTPUT_BYTES, PortScriptError};

pub(super) async fn port(spawned: Spawned) -> Result<ServicePort, PortScriptError> {
    let Spawned {
        child,
        stdin,
        stdout,
        stderr,
    } = spawned;
    // Port selection has no interactive protocol; EOF is part of its contract.
    drop(stdin);
    collect(child, stdout, stderr).await
}

/// Keep the child and both readers in one future, so cancellation drops all three.
pub(super) async fn collect(
    mut child: vessel_proc::Child,
    stdout: impl AsyncRead + Unpin,
    stderr: impl AsyncRead + Unpin,
) -> Result<ServicePort, PortScriptError> {
    let (stdout, _stderr, code) = tokio::try_join!(
        read(stdout, Stream::Stdout),
        read(stderr, Stream::Stderr),
        async { child.wait().await.map_err(PortScriptError::Wait) },
    )?;
    if code != Some(0) {
        return Err(PortScriptError::Exit(code));
    }
    decode(&stdout)
}

pub(super) async fn read(
    reader: impl AsyncRead + Unpin,
    stream: Stream,
) -> Result<Vec<u8>, PortScriptError> {
    // One extra byte distinguishes an exact-cap EOF from silent truncation.
    let mut bytes = Vec::with_capacity(usize::from(MAX_OUTPUT_BYTES).saturating_add(1));
    reader
        .take(u64::from(MAX_OUTPUT_BYTES).saturating_add(1))
        .read_to_end(&mut bytes)
        .await
        .map_err(|source| PortScriptError::Read { stream, source })?;
    if bytes.len() > usize::from(MAX_OUTPUT_BYTES) {
        return Err(PortScriptError::OutputLimit(stream));
    }
    Ok(bytes)
}

pub(super) fn decode(bytes: &[u8]) -> Result<ServicePort, PortScriptError> {
    let text = std::str::from_utf8(bytes).map_err(PortScriptError::Utf8)?;
    trim_project_text(text)
        .parse()
        .map_err(PortScriptError::Port)
}
