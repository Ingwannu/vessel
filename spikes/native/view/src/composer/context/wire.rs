//! A context pill as the prompt carries it (ADR 0124): a `context` block
//! when the node speaks `PROMPT_CONTEXT_PROTOCOL`, so the timeline draws it
//! as Paseo's sent pill, and the `text` block it always was otherwise.
//!
//! [Decision Log]
//! - Purpose: the coordinator's ADR 0124 ruling, item 5: clients send
//!   `context` to a node at the protocol, `text` to an older one.
//! - Constraint: the tray's bounds are the app's (a title of 1 KiB, the
//!   drafts' text bound); the wire's are tighter (200 characters, 64 KiB).
//! - Choice: a title past the wire's bound is cut to it with an ellipsis
//!   (it only labels the pill); a text past it, or a value the wire refuses,
//!   goes as `text`, so the agent reads every word either way. An unsent
//!   review draft (`ContextKind::Review`) goes as text until the review
//!   block's packet (P6b-2) sends it typed.

use vessel_proto::ContentBlock;
use vessel_proto::prompt_context::{
    self, ContextKind as WireKind, ContextText as WireText, ContextTitle as WireTitle,
    ContextUrl as WireUrl, PromptContext,
};

use super::{ContextAttachment, ContextKind, Selection};

/// The kind a wire block names; `None` for a kind sent as text.
const fn wire_kind(kind: ContextKind) -> Option<WireKind> {
    match kind {
        ContextKind::BrowserElement => Some(WireKind::BrowserElement),
        ContextKind::ChatHistory => Some(WireKind::ChatHistory),
        ContextKind::PullComment => Some(WireKind::PullComment),
        ContextKind::PullReview => Some(WireKind::PullReview),
        ContextKind::PullCheck => Some(WireKind::PullCheck),
        ContextKind::ForgeIssue => Some(WireKind::ForgeIssue),
        ContextKind::ForgeChangeRequest => Some(WireKind::ForgeChangeRequest),
        ContextKind::PluginResource => Some(WireKind::PluginResource),
        ContextKind::Review => None,
    }
}

/// `text` as a pill's label: at most the wire's characters, cut with an
/// ellipsis.
fn title(text: &str) -> Option<WireTitle> {
    let limit = prompt_context::TITLE_CHARS;
    if text.chars().count() <= limit {
        return text.parse().ok();
    }
    let cut: String = text.chars().take(limit.saturating_sub(1)).collect();
    format!("{}…", cut.trim_end()).parse().ok()
}

impl ContextAttachment {
    /// This pill in a prompt: a `context` block when `speaks` (the node is
    /// at `PROMPT_CONTEXT_PROTOCOL`) and the wire takes its values, a
    /// `text` block of [`Self::text`] otherwise.
    #[must_use]
    pub fn block(&self, speaks: bool) -> ContentBlock {
        if let Self::Review(offer) = self {
            return if speaks {
                ContentBlock::Review(offer.snapshot.review.clone())
            } else {
                ContentBlock::Text {
                    text: offer.snapshot.text(),
                }
            };
        }
        let text = self.text();
        let typed = speaks.then(|| self.context(&text)).flatten();
        typed.map_or(ContentBlock::Text { text }, ContentBlock::Context)
    }

    fn context(&self, text: &str) -> Option<PromptContext> {
        let text: WireText = text.parse().ok()?;
        match self {
            Self::WorkspaceFile(file) => {
                let path = file.path.as_str();
                let name = path.rsplit('/').next().unwrap_or(path);
                let place = match file.selection {
                    Selection::Whole => path.to_owned(),
                    Selection::Lines(lines) => {
                        format!("{path} · {}-{}", lines.start(), lines.end())
                    }
                };
                Some(PromptContext {
                    kind: WireKind::WorkspaceFile,
                    title: title(name)?,
                    subtitle: title(&place),
                    text,
                    url: None,
                })
            }
            Self::Review(_) => None,
            Self::Snapshot { kind, payload } => Some(PromptContext {
                kind: wire_kind(*kind)?,
                title: title(payload.title.as_str())?,
                subtitle: None,
                text,
                url: payload
                    .url
                    .as_ref()
                    .and_then(|url| url.as_str().parse::<WireUrl>().ok()),
            }),
        }
    }
}
