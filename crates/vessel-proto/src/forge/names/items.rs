//! The bounded values the pull request pane's reads carry (ADR 0088): a
//! forge's item id, a file path as a forge names it, and a forge's text
//! made a description-sized body. The same rules as `names`: strict when
//! decoded, lossy when the node builds one from a forge's answer.

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use super::{BODY, InvalidForgeValue, PullBody, invalid, line, lossy_line};
use crate::key::serde_via_text;

/// The most bytes of a forge's item id.
const ITEM: usize = 128;
/// The most bytes of a forge's file path.
const FORGE_PATH: usize = 4096;

forge_text!(
    /// A forge's own id of a review, a comment or a thread (GitHub's node
    /// ids): opaque, for keys and replies only.
    ForgeItemId,
    "forge item id",
    item_rule
);
forge_text!(
    /// A file's path in a repository as a forge names it (a thread's or an
    /// annotation's): one line, shown, never opened as a path.
    ForgePath,
    "forge file path",
    forge_path_rule
);

fn item_rule(text: &str) -> Result<(), &'static str> {
    if text.is_empty() || text.len() > ITEM {
        return Err("1 to 128 bytes");
    }
    if !text.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err("printable ASCII, no space");
    }
    Ok(())
}

fn forge_path_rule(text: &str) -> Result<(), &'static str> {
    if text.len() > FORGE_PATH {
        return Err("at most 4 KiB");
    }
    line(text, FORGE_PATH)
}

impl ForgeItemId {
    /// `text` when it is an id; `None` otherwise.
    #[must_use]
    pub fn lossy(text: &str) -> Option<Self> {
        text.parse().ok()
    }
}

impl ForgePath {
    /// A path made of a forge's `text`: one line, at most 4 KiB; `None`
    /// when nothing is left.
    #[must_use]
    pub fn lossy(text: &str) -> Option<Self> {
        lossy_line(text, FORGE_PATH)
            .filter(|line| line.len() <= FORGE_PATH)
            .map(|line| Self(Arc::from(line)))
    }
}

impl PullBody {
    /// A text made of a forge's `text` (a review's, a comment's, a check's
    /// output): NUL removed, cut at a character boundary to 64 KiB.
    #[must_use]
    pub fn lossy(text: &str) -> Self {
        let mut kept: String = text
            .chars()
            .filter(|character| *character != '\0')
            .collect();
        let mut end = kept.len().min(BODY);
        while !kept.is_char_boundary(end) {
            end = end.saturating_sub(1);
        }
        kept.truncate(end);
        Self(Arc::from(kept))
    }
}
