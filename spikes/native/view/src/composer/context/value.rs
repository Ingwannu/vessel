//! Validate context metadata before a panel can attach it to a draft.
use std::sync::Arc;

/// Context keys remain small even when an external producer uses a composite id.
pub(super) const MAX_KEY: usize = 1024;
/// A single-line pill title must not retain an unbounded provider response.
pub(super) const MAX_TITLE: usize = 1024;
/// A Unicode history snapshot fits within the existing total draft budget.
pub(super) const MAX_TEXT: usize = crate::composer::drafts::Drafts::MAX_TOTAL;
/// URLs are metadata rather than arbitrary retained context bodies.
pub(super) const MAX_URL: usize = 8192;

/// Why context input was refused before changing the tray.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ContextProblem {
    /// The current snapshot is still waiting for its submission result.
    #[error("context submission is already pending")]
    Busy,
    /// Correlation identities may never wrap into an older request.
    #[error("context correlation identities exhausted")]
    Exhausted,
    /// An identity is missing or too large.
    #[error("context identity must contain between 1 and {MAX_KEY} bytes without controls")]
    Identity,
    /// A title is empty, multiline or too large.
    #[error("context title must contain between 1 and {MAX_TITLE} bytes on one line")]
    Title,
    /// The text cannot be kept within the app's draft limit.
    #[error("context text exceeds {MAX_TEXT} bytes or contains a NUL")]
    Text,
    /// Only existing safe web links may be opened by a pill.
    #[error("context URL must be an absolute HTTP or HTTPS URL")]
    Url,
    /// Selection lines are one-based and cannot go backwards.
    #[error("context line range must be one-based and ordered")]
    Lines,
    /// The source and destination refer to different authorized workspaces.
    #[error("context belongs to a different machine or workspace")]
    Scope,
    /// The current draft already has its bounded number of context items.
    #[error("too many context attachments in one prompt")]
    Capacity,
}

/// A producer's opaque context identity; it is never decoded into another id.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContextId(Arc<str>);

impl ContextId {
    /// Validate a producer key without inventing its plugin/forge registry.
    ///
    /// # Errors
    /// Refuses invalid input or an unavailable transition with `ContextProblem`.
    pub fn parse(value: &str) -> Result<Self, ContextProblem> {
        if value.is_empty() || value.len() > MAX_KEY || value.chars().any(char::is_control) {
            return Err(ContextProblem::Identity);
        }
        Ok(Self(Arc::from(value)))
    }
    /// Borrow the retained opaque identity.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A bounded title shared by pills and context previews.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextTitle(Arc<str>);
impl ContextTitle {
    /// Keep a title only after its single-line contract is checked.
    ///
    /// # Errors
    /// Refuses invalid input or an unavailable transition with `ContextProblem`.
    pub fn parse(value: &str) -> Result<Self, ContextProblem> {
        if value.trim().is_empty() || value.len() > MAX_TITLE || value.chars().any(char::is_control)
        {
            return Err(ContextProblem::Title);
        }
        Ok(Self(Arc::from(value)))
    }
    /// A pill's borrowed visible title.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Plain agent context, independent of its source panel or renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextText(Arc<str>);
impl ContextText {
    /// Bound for a complete formatted snapshot, including any producer wrapper.
    pub const MAX_BYTES: usize = MAX_TEXT;
    /// Reject oversized or NUL-bearing input before it enters retained state.
    ///
    /// # Errors
    /// Refuses invalid input or an unavailable transition with `ContextProblem`.
    pub fn parse(value: &str) -> Result<Self, ContextProblem> {
        if value.len() > MAX_TEXT || value.contains('\0') {
            return Err(ContextProblem::Text);
        }
        Ok(Self(Arc::from(value)))
    }
    /// The exact text to send; no markup is stripped during submission.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A URL accepted by the existing untrusted-output link policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextUrl(Arc<str>);
impl ContextUrl {
    /// Apply the app's existing HTTP(S)-only policy once at intake.
    ///
    /// # Errors
    /// Refuses invalid input or an unavailable transition with `ContextProblem`.
    pub fn parse(value: &str) -> Result<Self, ContextProblem> {
        if value.len() > MAX_URL || !crate::links::is_web_url(value) {
            return Err(ContextProblem::Url);
        }
        Ok(Self(Arc::from(value)))
    }
    /// The retained URL; opening still goes through the app's link policy.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
