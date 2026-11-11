//! `describe`: what a prompt held, block by block (protocol 10's
//! attachments), so a test sees what reached the agent.

use std::path::PathBuf;

use agent_client_protocol::v1::schema::ContentBlock;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

/// One line: each block's kind and size, `; `-separated. An image or audio
/// block says its type and how many bytes its base64 decodes to; a
/// resource link its name, type, declared size, and how many bytes the
/// file its `file:` URI names holds (`unreadable` otherwise).
pub(crate) fn blocks(prompt: &[ContentBlock]) -> String {
    prompt
        .iter()
        .map(|block| match block {
            ContentBlock::Text(text) => format!("text {}", text.text.len()),
            ContentBlock::Image(image) => {
                format!("image {} {}", image.mime_type, decoded(&image.data))
            }
            ContentBlock::Audio(audio) => {
                format!("audio {} {}", audio.mime_type, decoded(&audio.data))
            }
            ContentBlock::ResourceLink(link) => format!(
                "link {} {} {} {}",
                link.name,
                link.mime_type.as_deref().unwrap_or("-"),
                link.size
                    .map_or_else(|| "-".to_owned(), |size| size.to_string()),
                read(&link.uri),
            ),
            ContentBlock::Resource(_) => "resource".to_owned(),
            _ => "other".to_owned(),
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn decoded(data: &str) -> String {
    STANDARD.decode(data).map_or_else(
        |_| "undecodable".to_owned(),
        |bytes| format!("{} bytes", bytes.len()),
    )
}

/// How many bytes the file `uri` names holds.
fn read(uri: &str) -> String {
    let path: Option<PathBuf> = url::Url::parse(uri)
        .ok()
        .and_then(|url| url.to_file_path().ok());
    path.and_then(|path| std::fs::read(path).ok()).map_or_else(
        || "unreadable".to_owned(),
        |bytes| format!("read {}", bytes.len()),
    )
}
