//! `vessel send`: a line for a terminal session; a prompt for a structured
//! one, delivered as asked (protocol 10) and with files attached.
//!
//! [Decision Log]
//! - Without a delivery or a file, `send` makes the call it always made
//!   (`session.send`), so it still works with a node before protocol 10;
//!   with either, `session.prompt`, whose `start` is that same delivery,
//!   and the outcome is printed (`started`, `steered`, `queued <entry>`).
//! - A file is read and sent a piece at a time (at most the node's chunk,
//!   `BlobChunk::MAX_LEN`), never held whole: files go up to 50 MiB. Its
//!   size is fixed when the upload begins; a file that ends early is
//!   refused here rather than finished short. Its upload is then left for
//!   the node to drop when idle: protocol 10 has no call to abandon one.
//! - A file's type comes from its extension: the image types the node
//!   gives an agent inline, common audio, a few text and document types,
//!   anything else `application/octet-stream`. A crate that guesses more
//!   types would only name more of what the agent reads as a file anyway.

use std::ffi::OsStr;
use std::path::Path;

use bytes::Bytes;
use tokio::io::AsyncReadExt as _;
use vessel_client::NodeClient;
use vessel_proto::conversation::{AttachmentName, MimeType, Upload};
use vessel_proto::{
    AttachmentRef, BlobChunk, ContentBlock, Delivery, SessionId, SessionInput, SessionMode,
    TerminalBytes, TerminalInput,
};

use crate::args::{Send, Target};
use crate::connect::connect;
use crate::error::CliError;
use crate::{io, render};

/// `vessel send`.
pub(crate) async fn send(target: &Target, args: Send) -> Result<(), CliError> {
    let client = connect(target).await?.client;
    let info = args.session.resolve(&client).await?;
    let delivery = args.delivery.delivery();
    let protocol_10 = delivery.is_some() || !args.attach.is_empty();
    match info.spec.mode {
        SessionMode::Terminal { .. } if protocol_10 => Err(CliError::Usage(
            "--queue, --steer, --interrupt and --attach are for structured sessions",
        )),
        // Ended with Enter, `\r`, which a TUI in raw mode and a line-mode
        // program both take as the end of a line.
        SessionMode::Terminal { .. } => {
            let input = SessionInput::Terminal {
                input: TerminalInput::Data {
                    data: TerminalBytes::new(Bytes::from(format!("{}\r", args.text))),
                },
            };
            Ok(client.send(info.id, input).await?)
        }
        SessionMode::Structured { .. } if !protocol_10 => {
            let content = vec![ContentBlock::Text { text: args.text }];
            Ok(client
                .send(info.id, SessionInput::Prompt { content })
                .await?)
        }
        SessionMode::Structured { .. } => {
            let mut content = Vec::with_capacity(args.attach.len().saturating_add(1));
            if !args.text.is_empty() {
                content.push(ContentBlock::Text { text: args.text });
            }
            for path in &args.attach {
                let attachment = upload(&client, info.id, path).await?;
                content.push(ContentBlock::Attachment { attachment });
            }
            let delivered = client
                .prompt(info.id, content, delivery.unwrap_or(Delivery::Start))
                .await?;
            let line = format!("{}\n", render::delivered(&delivered));
            Ok(io::out(line.as_bytes()).await?)
        }
    }
}

/// Upload the file at `path` as an attachment of `session`, a piece at a
/// time; the reference a prompt names it by.
async fn upload(
    client: &NodeClient,
    session: SessionId,
    path: &Path,
) -> Result<AttachmentRef, CliError> {
    let unreadable = |source| CliError::Attach {
        path: path.to_owned(),
        source,
    };
    let not_attachable = |source| CliError::NotAttachable {
        path: path.to_owned(),
        source,
    };
    let name: AttachmentName = path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .parse()
        .map_err(not_attachable)?;
    let mime: MimeType = mime(path).parse().map_err(not_attachable)?;
    let mut file = tokio::fs::File::open(path).await.map_err(unreadable)?;
    let size = file.metadata().await.map_err(unreadable)?.len();
    let Upload { upload, chunk } = client.begin_upload(session, name, mime, size).await?;
    let piece = usize::try_from(chunk)
        .unwrap_or(BlobChunk::MAX_LEN)
        .clamp(1, BlobChunk::MAX_LEN);
    let mut offset = 0_u64;
    while offset < size {
        let left = usize::try_from(size.saturating_sub(offset)).unwrap_or(usize::MAX);
        // One buffer per piece: the piece is handed over to be sent.
        let mut buffer = vec![0_u8; piece.min(left)];
        let _filled = file.read_exact(&mut buffer).await.map_err(|error| {
            if error.kind() == std::io::ErrorKind::UnexpectedEof {
                CliError::AttachmentChanged(path.to_owned())
            } else {
                unreadable(error)
            }
        })?;
        let length = u64::try_from(buffer.len()).unwrap_or(u64::MAX);
        let _written = client
            .write_upload(upload, offset, Bytes::from(buffer))
            .await?;
        offset = offset.saturating_add(length);
    }
    Ok(client.finish_upload(upload).await?)
}

/// The media type of the file at `path`, from its extension.
fn mime(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase);
    match extension.as_deref() {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("ogg" | "oga" | "opus") => "audio/ogg",
        Some("flac") => "audio/flac",
        Some("m4a") => "audio/mp4",
        Some("txt" | "log") => "text/plain",
        Some("md" | "markdown") => "text/markdown",
        Some("csv") => "text/csv",
        Some("html" | "htm") => "text/html",
        Some("json") => "application/json",
        Some("pdf") => "application/pdf",
        Some("zip") => "application/zip",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use vessel_proto::AttachmentKind;

    use super::*;

    #[test]
    fn a_files_type_follows_its_extension_and_parses() {
        let kind = |name: &str| AttachmentKind::of(&mime(Path::new(name)).parse().unwrap());
        assert_eq!(kind("shot.PNG"), AttachmentKind::Image);
        assert_eq!(kind("photo.jpeg"), AttachmentKind::Image);
        assert_eq!(kind("voice.m4a"), AttachmentKind::Audio);
        assert_eq!(kind("notes.md"), AttachmentKind::File);
        assert_eq!(mime(Path::new("Makefile")), "application/octet-stream");
        assert_eq!(
            mime(Path::new("archive.tar.gz")),
            "application/octet-stream"
        );
        for name in [
            "a.png", "a.jpg", "a.gif", "a.webp", "a.mp3", "a.wav", "a.ogg", "a.flac", "a.m4a",
            "a.txt", "a.md", "a.csv", "a.html", "a.json", "a.pdf", "a.zip", "a",
        ] {
            assert!(mime(Path::new(name)).parse::<MimeType>().is_ok(), "{name}");
        }
    }
}
