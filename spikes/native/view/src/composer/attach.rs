//! What may be attached to a prompt, decided before a byte is sent (track
//! P's row c): a file's type by its name, its kind as the node decides it,
//! whether the agent takes that kind, whether it fits the node's limits,
//! and the name the node will accept.
//!
//! [Decision Log]
//! - Purpose: a file the node would refuse is refused in the composer, at
//!   once and with its reason, not after it was uploaded (Paseo checks a
//!   file against its daemon's upload limit before sending it,
//!   `attachments/upload-limit.ts`).
//! - Constraint: the node is the authority (ADR 0025: it checks again at
//!   `attachment.begin` and when the prompt names the file, and counts
//!   what the session already holds, which the app does not know). The
//!   rules here are the node's, read from the same places: the kind from
//!   the type (`AttachmentKind::of`), the agent's `abilities`, the
//!   conversation's `limits`.
//! - Followed: Vessel's own page (`web/src/data/attach.ts`), which ports the
//!   same rules, value for value; the type table is the CLI's
//!   (`vessel attach`), so a file is the same type whichever client sends
//!   it.
//! - Choice: pure functions, each tested; the tray (`tray.rs`) calls them
//!   as files arrive. What only the node can know (the session's own
//!   total, the node's whole store) stays the node's: its refusal is shown
//!   on the file's pill.

use vessel_proto::conversation::MimeType;
use vessel_proto::{AgentAbilities, AttachmentKind, AttachmentLimits, AttachmentName};

/// A destination address may disappear or be rebound to a different node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TargetProblem {
    /// Same node identity is insufficient after an admission retires.
    #[error(transparent)]
    Admission(#[from] crate::connection::Refusal),
    /// No verified peer is available under the captured address.
    #[error("the attachment destination is offline")]
    Offline,
    /// Never route queued bytes to a replacement node with the same book name.
    #[error("the attachment destination identity changed")]
    Changed {
        /// The admitted welcome identity.
        expected: vessel_proto::NodeId,
        /// The replacement peer which must receive no side effect.
        actual: vessel_proto::NodeId,
    },
}

/// Preparation refuses before changing the retained draft.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PrepareProblem {
    /// Another creation owns the existing snapshot.
    #[error("attachment creation is already pending")]
    Busy,
    /// All bounded submission slots are retained.
    #[error("attachment preparation capacity reached")]
    Capacity,
    /// Preserve the existing machine-name validator result.
    #[error("invalid machine selection: {0:?}")]
    Machine(crate::machines::NameProblem),
    /// Context validation preserves its source-specific refusal.
    #[error(transparent)]
    Context(#[from] super::context::ContextProblem),
    /// A peer has not provided an authoritative destination identity.
    #[error(transparent)]
    Target(#[from] TargetProblem),
}

/// The longest file name the node takes, in UTF-8 bytes.
const NAME_MAX: usize = 255;

/// The longest extension kept when a name is cut, dot included.
const EXTENSION_MAX: usize = 16;

/// Characters no file name may hold on some platform the node runs on.
const FORBIDDEN: [char; 9] = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

/// The type the node is told for a file it cannot place.
const UNKNOWN: &str = "application/octet-stream";

/// The type of a file named `name`, from its extension (the CLI's table);
/// `application/octet-stream` for any other. `None` only if a type of the
/// table were not one the node takes (a test holds that none is).
#[must_use]
pub fn mime_of(name: &str) -> Option<MimeType> {
    let extension = name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase());
    let mime = match extension.as_deref() {
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
        _ => UNKNOWN,
    };
    mime.parse().ok()
}

/// The kind the node gives a file of type `mime`.
#[must_use]
pub fn kind_of(mime: &MimeType) -> AttachmentKind {
    AttachmentKind::of(mime)
}

/// Windows opens a device for these stems, whatever the extension.
fn is_device(stem: &str) -> bool {
    let lower = stem.to_lowercase();
    let numbered = |prefix: &str| {
        lower.strip_prefix(prefix).is_some_and(|rest| {
            let mut characters = rest.chars();
            matches!(
                (characters.next(), characters.next()),
                (Some('0'..='9' | '¹' | '²' | '³'), None)
            )
        })
    };
    matches!(
        lower.as_str(),
        "con" | "prn" | "aux" | "nul" | "conin$" | "conout$"
    ) || numbered("com")
        || numbered("lpt")
}

/// `raw` as the node accepts a file's name ([`AttachmentName`]): one path
/// component; no control character nor any of `/ \ : * ? " < > |` (each
/// becomes `_`); not starting with a space or a dot (`.env` becomes
/// `_.env`), not ending with one; not a Windows device; at most 255 bytes,
/// cut on a character before a short extension. `fallback` when nothing is
/// left; `None` only when `fallback` is not a name either.
#[must_use]
pub fn attachment_name(raw: &str, fallback: &str) -> Option<AttachmentName> {
    let mut cleaned: String = raw
        .chars()
        .map(|character| {
            if character.is_control() || FORBIDDEN.contains(&character) {
                '_'
            } else {
                character
            }
        })
        .collect();
    let kept = cleaned
        .trim_end_matches(|character: char| character.is_whitespace() || character == '.')
        .len();
    cleaned.truncate(kept);
    if cleaned.starts_with(|character: char| character.is_whitespace() || character == '.') {
        cleaned.insert(0, '_');
    }
    let stem = cleaned.split('.').next().unwrap_or_default().trim_end();
    if is_device(stem) {
        cleaned.insert(0, '_');
    }
    fitted(&cleaned).parse().or_else(|_| fallback.parse()).ok()
}

/// `name` cut to [`NAME_MAX`] bytes on a character boundary, keeping its
/// extension where it has a short one.
fn fitted(name: &str) -> String {
    if name.len() <= NAME_MAX {
        return name.to_owned();
    }
    let (stem, extension) = match name.rfind('.') {
        Some(dot) if dot > 0 && name.len().saturating_sub(dot) <= EXTENSION_MAX => {
            name.split_at(dot)
        }
        _ => (name, ""),
    };
    let room = NAME_MAX.saturating_sub(extension.len());
    let mut end = room.min(stem.len());
    while !stem.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    let cut = stem.get(..end).unwrap_or_default();
    format!("{cut}{extension}")
        .trim_end_matches(|character: char| character.is_whitespace() || character == '.')
        .to_owned()
}

/// Whether `head`, a file's first bytes, starts as an image of type `mime`
/// does (the node checks the same at `attachment.finish`); any other type
/// passes.
#[must_use]
pub fn looks_like(mime: &MimeType, head: &[u8]) -> bool {
    match mime.as_str() {
        "image/png" => head.starts_with(&[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
        "image/jpeg" => head.starts_with(&[0xff, 0xd8, 0xff]),
        "image/gif" => head.starts_with(b"GIF8"),
        "image/webp" => head.starts_with(b"RIFF") && head.get(8..12) == Some(b"WEBP".as_slice()),
        _ => true,
    }
}

/// A file as the rules weigh it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candidate {
    /// What it is to the agent.
    pub kind: AttachmentKind,
    /// Its size in bytes.
    pub size: u64,
}

/// Why a file is not attached, before anything is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The agent does not take this kind (it said so in its handshake).
    Agent(AttachmentKind),
    /// Larger than a file of its kind may be, in bytes.
    TooLarge {
        /// The limit.
        limit: u64,
    },
    /// An empty file.
    Empty,
    /// One more than a prompt may name.
    TooMany {
        /// The limit.
        limit: u32,
    },
    /// Its images and audio together are more than a prompt may carry.
    InlineTotal {
        /// The limit, in bytes.
        limit: u64,
    },
}

/// Whether `candidate` may join `held` (the files already attached and not
/// refused) under the agent's `abilities` and the node's `limits`; `None`
/// when it may.
#[must_use]
pub fn refusal(
    candidate: Candidate,
    held: &[Candidate],
    abilities: Option<&AgentAbilities>,
    limits: &AttachmentLimits,
) -> Option<Refusal> {
    if !takes(abilities, candidate.kind) {
        return Some(Refusal::Agent(candidate.kind));
    }
    if candidate.size == 0 {
        return Some(Refusal::Empty);
    }
    let limit = candidate.kind.max_size(limits);
    if candidate.size > limit {
        return Some(Refusal::TooLarge { limit });
    }
    let count = u32::try_from(held.len()).unwrap_or(u32::MAX);
    if count >= limits.per_prompt {
        return Some(Refusal::TooMany {
            limit: limits.per_prompt,
        });
    }
    if candidate.kind.is_inline() {
        let inline = held
            .iter()
            .filter(|other| other.kind.is_inline())
            .fold(candidate.size, |sum, other| sum.saturating_add(other.size));
        if inline > limits.inline_per_prompt {
            return Some(Refusal::InlineTotal {
                limit: limits.inline_per_prompt,
            });
        }
    }
    None
}

/// Whether the agent takes `kind`: files always (a link to a stored file,
/// ACP's baseline), images and audio when its handshake says so.
#[must_use]
pub fn takes(abilities: Option<&AgentAbilities>, kind: AttachmentKind) -> bool {
    match kind {
        AttachmentKind::File => true,
        AttachmentKind::Image => abilities.is_some_and(|abilities| abilities.image),
        AttachmentKind::Audio => abilities.is_some_and(|abilities| abilities.audio),
    }
}

#[cfg(test)]
mod tests;
