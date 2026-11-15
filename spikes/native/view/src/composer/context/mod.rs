//! Context from other panels shares the attachment tray without copying
//! their workspace, forge or plugin registries. Producers retain their
//! canonical identities and hand over a validated immutable text snapshot.

mod store;
mod value;
use crate::machine_book::Name;
use std::num::NonZeroU32;
pub use store::{ContextTray, ContextVisibility, Submission, SubmissionOutcome};
pub use value::{ContextId, ContextProblem, ContextText, ContextTitle, ContextUrl};
use vessel_proto::workspace::{RelPath, WorkspaceRoot};

/// Scope is the existing authorized node/root, never a new `WorkspaceId`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextScope {
    /// The validated machine-book identity.
    pub machine: Name,
    /// The same root the file/forge action was authorized against.
    pub root: WorkspaceRoot,
}

/// A validated one-based range, including both endpoint lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineRange {
    start: NonZeroU32,
    end: NonZeroU32,
}
impl LineRange {
    /// Reject empty, reversed or zero-based selections before attaching.
    ///
    /// # Errors
    /// Refuses invalid input or an unavailable transition with `ContextProblem`.
    pub fn new(start: u32, end: u32) -> Result<Self, ContextProblem> {
        let start = NonZeroU32::new(start).ok_or(ContextProblem::Lines)?;
        let end = NonZeroU32::new(end)
            .filter(|end| *end >= start)
            .ok_or(ContextProblem::Lines)?;
        Ok(Self { start, end })
    }
    /// First included line.
    #[must_use]
    pub fn start(self) -> NonZeroU32 {
        self.start
    }
    /// Last included line.
    #[must_use]
    pub fn end(self) -> NonZeroU32 {
        self.end
    }
}

/// File attachment selection follows Paseo's whole-file or line-range form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Selection {
    /// Refer to the entire file without reading its content.
    Whole,
    /// Refer only to these lines, still as a pointer rather than copied content.
    Lines(LineRange),
}

/// A file selected in an already authorized workspace panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceFile {
    /// Source scope, checked again against the destination before insertion.
    pub scope: ContextScope,
    /// Validated path confined to that root.
    pub path: RelPath,
    /// Whole file or one-based lines.
    pub selection: Selection,
}

/// An immutable snapshot received from an external feature owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payload {
    /// The producer's opaque key, not a replacement plugin/forge identity.
    pub id: ContextId,
    /// The visible pill label.
    pub title: ContextTitle,
    /// Text which the agent receives without fetching an arbitrary URL.
    pub text: ContextText,
    /// Optional source page, subject to the shared link-opening policy.
    pub url: Option<ContextUrl>,
}

/// Fork seeding has history before it has a registered destination root.
/// The producer owns recent-history selection and its explicit truncation state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatHistory(Payload);
impl ChatHistory {
    /// Every field in this snapshot has already passed the shared validators.
    #[must_use]
    pub const fn new(payload: Payload) -> Self {
        Self(payload)
    }
    /// Preserve the history semantic kind without string dispatch at the intake.
    #[must_use]
    pub fn into_attachment(self) -> ContextAttachment {
        ContextAttachment::Snapshot {
            kind: ContextKind::ChatHistory,
            payload: self.0,
        }
    }
}

/// Typed distinctions affect labels, replacement and suppression semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextKind {
    /// Browser element context (including its already formatted text).
    BrowserElement,
    /// An unsent review draft.
    Review,
    /// A selected conversation history boundary.
    ChatHistory,
    /// One pull-request comment.
    PullComment,
    /// One pull-request review.
    PullReview,
    /// One pull-request check's output.
    PullCheck,
    /// A forge issue snapshot.
    ForgeIssue,
    /// A forge change request snapshot.
    ForgeChangeRequest,
}

/// Every non-byte attachment is explicit; image/file bytes stay in the
/// existing upload tray. Feature owners adapt canonical source types here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextAttachment {
    /// Canonical captured plugin/source/item identity and validated retained text.
    Plugin(vessel_proto::plugins::ContextAttachment),
    /// File explorer and diff actions supply this pointer.
    WorkspaceFile(WorkspaceFile),
    /// Other panels provide an immutable text snapshot and its semantic kind.
    Snapshot {
        /// Preserve kind without string dispatch.
        kind: ContextKind,
        /// The validated snapshot and source key.
        payload: Payload,
    },
}

impl ContextAttachment {
    /// Context is sent through existing text blocks, never a fabricated RPC.
    #[must_use]
    pub fn text(&self) -> String {
        match self {
            Self::Plugin(item) => item.item.text.as_str().to_owned(),
            Self::WorkspaceFile(file) => match file.selection {
                Selection::Whole => format!("Workspace file: {}", file.path.as_str()),
                Selection::Lines(lines) => format!(
                    "Workspace file: {}\nLines: {}-{}",
                    file.path.as_str(),
                    lines.start(),
                    lines.end()
                ),
            },
            Self::Snapshot { payload, .. } => payload.text.as_str().to_owned(),
        }
    }

    /// A context pill's title; paths retain their full form in its subtitle.
    #[must_use]
    pub fn title(&self) -> &str {
        match self {
            Self::Plugin(item) => item.item.title.as_str(),
            Self::WorkspaceFile(file) => file
                .path
                .as_str()
                .rsplit('/')
                .next()
                .unwrap_or(file.path.as_str()),
            Self::Snapshot { payload, .. } => payload.title.as_str(),
        }
    }

    fn same_key(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Plugin(a), Self::Plugin(b)) => a.key() == b.key(),
            (Self::WorkspaceFile(a), Self::WorkspaceFile(b)) => a == b,
            (
                Self::Snapshot {
                    kind: a,
                    payload: x,
                },
                Self::Snapshot {
                    kind: b,
                    payload: y,
                },
            ) => a == b && x.id == y.id,
            (Self::Plugin(_), Self::WorkspaceFile(_) | Self::Snapshot { .. })
            | (Self::WorkspaceFile(_) | Self::Snapshot { .. }, Self::Plugin(_))
            | (Self::WorkspaceFile(_), Self::Snapshot { .. })
            | (Self::Snapshot { .. }, Self::WorkspaceFile(_)) => false,
        }
    }
}

#[cfg(test)]
mod tests;
