//! Folder suggestions from the node user's home for Add project, and a
//! project's own icon (ADR 0112, Paseo's `directory_suggestions_request`
//! without a `cwd` and `project_icon_request`).
//!
//! [Decision Log]
//! - Purpose: Add project suggests folders as Paseo's does, and projects
//!   are drawn with their own icon.
//! - Constraint: `file.suggest` (ADR 0083) searches inside a registered
//!   root only and keeps that rule; Add project starts outside every
//!   project. A Read principal must not learn the node's folder tree.
//! - Alternatives: widen `file.suggest` to take no root (one method, two
//!   permission rules); search on the client's disk (wrong for every
//!   remote node).
//! - Choice: two methods under one protocol number. `folder.suggest`
//!   (Operate, what `project.add` already lets such a principal name):
//!   folders only, under the home, absolute paths with the home shown as
//!   `~`, never a file name. `project.icon` (Read): an image inside the
//!   project's root, its type decided by its bytes, square, at most
//!   [`MAX_ICON_BYTES`].
//! - Cost: a home walk per typed query (bounded in entries and time on the
//!   node); the home's folder names are visible to Operate principals.

use std::path::PathBuf;

use base64::Engine as _;
use bytes::Bytes;
use serde::{Deserialize, Serialize};

use super::suggestions::{SuggestionLimit, SuggestionQuery};
use crate::ProtocolVersion;
use crate::id::ProjectId;

/// The protocol of `folder.suggest` and `project.icon` (ADR 0112), both at
/// once: each client hides the suggestions and draws the folder mark on an
/// older peer. Drafted as the next free number; the integrator assigns it
/// in merge order.
pub const HOST_FOLDERS_PROTOCOL: Option<ProtocolVersion> = Some(ProtocolVersion::new(32));

/// Paseo's icon budget (`project-icon.ts` `MAX_ICON_SIZE`).
pub const MAX_ICON_BYTES: usize = 32 * 1024;
/// Encoded length checked before decoding a peer's icon.
const MAX_ICON_BASE64_CHARS: usize = MAX_ICON_BYTES.div_ceil(3) * 4;

/// `folder.suggest`: folders under the node user's home matching `query`.
/// A query that is an absolute or `~/` path lists that folder's subfolders
/// when it lies under the home (Paseo's exact-path retrieval); a blank
/// query suggests nothing (Paseo's `blankQueryBehavior: "none"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SuggestFolders {
    /// What was typed, bounded and one line.
    pub query: SuggestionQuery,
    /// At most this many (Paseo's 30 when absent, 100 at most).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<SuggestionLimit>,
}

/// One suggested folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SuggestedFolder {
    /// Absolute, as `project.add` takes it.
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub path: PathBuf,
    /// The same path with the home written `~`, for drawing.
    pub shown: String,
}

/// `folder.suggest`'s answer, best match first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FolderSuggestions {
    /// The folders.
    pub folders: Vec<SuggestedFolder>,
    /// The walk stopped at a bound (entries, depth, time) or the limit
    /// cut matches.
    pub more: bool,
}

/// `project.icon`: the registered project's own icon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetProjectIcon {
    /// Which project.
    pub project: ProjectId,
}

/// What an icon's bytes are, decided from the bytes (never the file name).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum IconType {
    /// PNG (an ICO's largest PNG frame is sent as PNG, as Paseo's).
    Png,
    /// An ICO container without a PNG frame.
    Ico,
    /// SVG text; clients draw it only as an image, never as markup.
    Svg,
}

impl IconType {
    /// The content type a page serves it with.
    #[must_use]
    pub const fn content_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Ico => "image/x-icon",
            Self::Svg => "image/svg+xml",
        }
    }
}

/// An icon's bytes: base64 on the wire, at most [`MAX_ICON_BYTES`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
pub struct IconBytes(Bytes);

/// An icon larger than [`MAX_ICON_BYTES`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a project icon is at most {MAX_ICON_BYTES} bytes")]
pub struct InvalidIconBytes;

impl IconBytes {
    /// Refuse an oversized icon where it is made or decoded.
    ///
    /// # Errors
    /// [`InvalidIconBytes`] beyond [`MAX_ICON_BYTES`].
    pub fn new(bytes: Bytes) -> Result<Self, InvalidIconBytes> {
        if bytes.len() > MAX_ICON_BYTES {
            return Err(InvalidIconBytes);
        }
        Ok(Self(bytes))
    }

    /// The bytes.
    #[must_use]
    pub const fn bytes(&self) -> &Bytes {
        &self.0
    }
}

impl Serialize for IconBytes {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        crate::base64_bytes::serialize(&self.0, serializer)
    }
}

impl<'de> Deserialize<'de> for IconBytes {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(IconBytesVisitor)
    }
}

struct IconBytesVisitor;

impl serde::de::Visitor<'_> for IconBytesVisitor {
    type Value = IconBytes;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "at most {MAX_ICON_BYTES} bytes encoded as base64"
        )
    }

    fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<Self::Value, E> {
        if text.len() > MAX_ICON_BASE64_CHARS {
            return Err(E::custom(InvalidIconBytes));
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(text)
            .map_err(E::custom)?;
        IconBytes::new(Bytes::from(bytes)).map_err(E::custom)
    }
}

/// A project's icon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct IconImage {
    /// What the bytes are.
    #[serde(rename = "type")]
    pub kind: IconType,
    /// The bytes.
    pub bytes: IconBytes,
}

/// `project.icon`'s answer: the icon, or none (no file of Paseo's names,
/// not square, too large, or bytes of no allowed type).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ProjectIcon {
    /// The icon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<IconImage>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_icon_is_base64_and_bounded_on_both_ends() {
        let icon = IconImage {
            kind: IconType::Png,
            bytes: IconBytes::new(Bytes::from_static(b"\x89PNG")).unwrap(),
        };
        let text = serde_json::to_string(&icon).unwrap();
        assert_eq!(text, r#"{"type":"png","bytes":"iVBORw=="}"#);
        assert_eq!(serde_json::from_str::<IconImage>(&text).unwrap(), icon);
        assert!(IconBytes::new(Bytes::from(vec![0; MAX_ICON_BYTES + 1])).is_err());
        let large = base64::engine::general_purpose::STANDARD.encode(vec![0; MAX_ICON_BYTES + 1]);
        assert!(serde_json::from_str::<IconBytes>(&format!("\"{large}\"")).is_err());
        assert_eq!(IconType::Svg.content_type(), "image/svg+xml");
        let none: ProjectIcon = serde_json::from_str("{}").unwrap();
        assert_eq!(none.icon, None);
    }
}
