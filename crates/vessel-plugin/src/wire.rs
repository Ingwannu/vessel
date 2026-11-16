//! Bounded length-prefixed JSON; arbitrary child errors cannot reach tracing.

use std::io;

use serde::{Serialize, de::DeserializeOwned};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use vessel_proto::plugins::{HelperToNode, MAX_HELPER_FRAME_BYTES, NodeToHelper, PluginJsonError};

/// Pipe/protocol failure without arbitrary helper body or local path text.
#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    /// A declared or encoded length exceeded the one framing budget.
    #[error("plugin helper frame must contain 1..={MAX_HELPER_FRAME_BYTES} bytes")]
    Length,
    /// Only OS errors from direct pipe reads/writes, with no helper text.
    #[error("plugin helper pipe failed")]
    Io(#[source] io::Error),
    /// Safe concrete parser cause with a sealed raw source chain.
    #[error("plugin helper JSON failed validation")]
    Json(#[source] PluginJsonError),
}

/// Decode a bounded complete helper body, useful also for adapter conformance
/// fixtures that do not start a process.
///
/// # Errors
/// Refuses malformed, unsupported or excessive messages without quoting input.
pub fn decode_helper(body: &[u8]) -> Result<HelperToNode, FrameError> {
    decode(body)
}

/// Decode the node side using the identical framing contract.
///
/// # Errors
/// Refuses malformed, unsupported or excessive messages without quoting input.
pub fn decode_node(body: &[u8]) -> Result<NodeToHelper, FrameError> {
    decode(body)
}

fn decode<T: DeserializeOwned>(body: &[u8]) -> Result<T, FrameError> {
    if body.is_empty() || body.len() > MAX_HELPER_FRAME_BYTES {
        return Err(FrameError::Length);
    }
    serde_json::from_slice(body).map_err(|cause| FrameError::Json(PluginJsonError::new(cause)))
}

/// The node reads only helper messages. Clean EOF is distinct from an incomplete
/// header/body; either withdraws live capabilities, but diagnostics stay precise.
///
/// # Errors
/// Fails on an incomplete, excessive, malformed or unsupported frame. Cancelling
/// this read retires the pipe; a subsequent read cannot restart a partial header.
pub async fn read_helper<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<Option<HelperToNode>, FrameError> {
    read(reader).await
}

/// The adapter reads only node messages.
///
/// # Errors
/// Fails on an incomplete, excessive, malformed or unsupported frame.
pub async fn read_node<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<Option<NodeToHelper>, FrameError> {
    read(reader).await
}

async fn read<R: AsyncRead + Unpin, T: DeserializeOwned>(
    reader: &mut R,
) -> Result<Option<T>, FrameError> {
    let mut first = [0_u8; 1];
    if reader.read(&mut first).await.map_err(FrameError::Io)? == 0 {
        return Ok(None);
    }
    let mut rest = [0_u8; 3];
    let _read = reader.read_exact(&mut rest).await.map_err(FrameError::Io)?;
    let [a] = first;
    let [b, c, d] = rest;
    let length =
        usize::try_from(u32::from_be_bytes([a, b, c, d])).map_err(|_range| FrameError::Length)?;
    if length == 0 || length > MAX_HELPER_FRAME_BYTES {
        return Err(FrameError::Length);
    }
    let mut body = vec![0; length];
    let _read = reader.read_exact(&mut body).await.map_err(FrameError::Io)?;
    decode(&body).map(Some)
}

/// Write one bounded node message and flush it as a complete frame.
///
/// # Errors
/// Fails on a serialization, size or pipe error. Callers must bound write time;
/// cancelling a partial write retires the whole pipe, never retries its suffix.
pub async fn write_node<W: AsyncWrite + Unpin>(
    writer: &mut W,
    message: &NodeToHelper,
) -> Result<(), FrameError> {
    write(writer, message).await
}

/// Write one bounded helper message with the same no-retry cancellation rule.
///
/// # Errors
/// Fails on a serialization, size or pipe error.
pub async fn write_helper<W: AsyncWrite + Unpin>(
    writer: &mut W,
    message: &HelperToNode,
) -> Result<(), FrameError> {
    write(writer, message).await
}

async fn write<W: AsyncWrite + Unpin, T: Serialize>(
    writer: &mut W,
    message: &T,
) -> Result<(), FrameError> {
    let mut body = BoundedBody {
        bytes: Vec::new(),
        exceeded: false,
    };
    if let Err(cause) = serde_json::to_writer(&mut body, message) {
        return Err(if body.exceeded {
            FrameError::Length
        } else {
            FrameError::Json(PluginJsonError::new(cause))
        });
    }
    let length = u32::try_from(body.bytes.len()).map_err(|_range| FrameError::Length)?;
    writer
        .write_all(&length.to_be_bytes())
        .await
        .map_err(FrameError::Io)?;
    writer
        .write_all(&body.bytes)
        .await
        .map_err(FrameError::Io)?;
    writer.flush().await.map_err(FrameError::Io)
}

struct BoundedBody {
    bytes: Vec<u8>,
    exceeded: bool,
}

impl io::Write for BoundedBody {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self
            .bytes
            .len()
            .checked_add(bytes.len())
            .is_none_or(|length| length > MAX_HELPER_FRAME_BYTES)
        {
            self.exceeded = true;
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "helper frame limit",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "wire_tests.rs"]
mod tests;
