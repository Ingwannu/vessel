//! Suppression is local to the composer; producers keep their source data.
//! A completion removes only the exact snapshots it sent, so a refresh or
//! a newly attached review cannot be erased by an older request finishing.

use super::{ContextAttachment, ContextKind, ContextProblem};
use std::sync::Arc;

/// Removing a workspace pill suppresses it rather than deleting source data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextVisibility {
    /// Included in the tray and the next submission.
    Visible,
    /// Hidden locally until submission failure or explicit reattachment.
    Suppressed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    identity: Arc<()>,
    revision: u64,
    attachment: ContextAttachment,
    visibility: ContextVisibility,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Idle,
    Submitting(u64),
}

/// One composer's retained context and its correlated submission phase.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContextTray {
    entries: Vec<Entry>,
    clock: u64,
    phase: Phase,
}

/// An immutable snapshot retained until the send is accepted or refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Submission {
    token: u64,
    entries: Vec<Entry>,
}

/// Queue acceptance consumes the submitted snapshot just as direct send does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmissionOutcome {
    /// The node accepted the prompt.
    Sent,
    /// The node accepted it into the message queue.
    Queued,
    /// Restore suppressed context after a refused or unsent prompt.
    Failed,
}

impl Submission {
    /// Only a typed snapshot can be converted to outgoing text blocks.
    pub fn attachments(&self) -> impl Iterator<Item = &ContextAttachment> {
        self.entries.iter().map(|entry| &entry.attachment)
    }
}

/// Captured entry incarnation and revision; matching resource keys alone do
/// not authorize a callback after a producer replaces or reattaches context.
#[derive(Debug, Clone)]
pub struct ContextIntent {
    identity: Arc<()>,
    revision: u64,
}

impl ContextTray {
    /// Drawn actions keep the actual entry, independently of its visible index.
    pub fn visible_intents(&self) -> impl Iterator<Item = (ContextIntent, &ContextAttachment)> {
        self.entries
            .iter()
            .filter(|entry| entry.visibility == ContextVisibility::Visible)
            .map(|entry| {
                (
                    ContextIntent {
                        identity: Arc::clone(&entry.identity),
                        revision: entry.revision,
                    },
                    &entry.attachment,
                )
            })
    }
    /// Resolve only the still-visible original incarnation and source revision.
    #[must_use]
    pub fn resolve(&self, intent: &ContextIntent) -> Option<&ContextAttachment> {
        self.entries
            .iter()
            .find(|entry| {
                entry.visibility == ContextVisibility::Visible
                    && entry.revision == intent.revision
                    && Arc::ptr_eq(&entry.identity, &intent.identity)
            })
            .map(|entry| &entry.attachment)
    }

    /// Suppressed source data still belongs to this draft and must block seeding.
    #[must_use]
    pub fn has_retained(&self) -> bool {
        !self.entries.is_empty() || self.phase != Phase::Idle
    }

    /// A background create transfers this exact snapshot to its new composer.
    /// Its token and revisions remain intact; later source updates stay behind.
    #[must_use]
    pub fn captured(submission: &Submission) -> Self {
        Self {
            entries: submission.entries.clone(),
            clock: submission.token,
            phase: Phase::Submitting(submission.token),
        }
    }

    /// Visible items retain insertion order; a replaced context moves last.
    pub fn visible(&self) -> impl Iterator<Item = &ContextAttachment> {
        self.entries
            .iter()
            .filter(|entry| entry.visibility == ContextVisibility::Visible)
            .map(|entry| &entry.attachment)
    }

    /// Whether the composer has any context to submit, independent of files.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.visible().next().is_none()
    }

    /// Dedupe files and selected external resources; replace chat/PR context
    /// with the same typed kind/id while retaining deliberate suppression.
    ///
    /// # Errors
    /// Refuses invalid input or an unavailable transition with `ContextProblem`.
    pub fn add(&mut self, attachment: ContextAttachment) -> Result<(), ContextProblem> {
        let matching = self
            .entries
            .iter()
            .position(|entry| entry.attachment.same_key(&attachment));
        let mut visibility = ContextVisibility::Visible;
        if matching.is_some() && !replaceable(&attachment) {
            return Ok(());
        }
        if matching.is_none()
            && u32::try_from(self.entries.len()).unwrap_or(u32::MAX)
                >= vessel_proto::AttachmentLimits::DEFAULT.per_prompt
        {
            return Err(ContextProblem::Capacity);
        }
        // Reserve correlation before changing retained source data.
        let revision = self.next()?;
        if let Some(index) = matching {
            if let Some(previous) = self.entries.get(index) {
                visibility = previous.visibility;
            }
            let _previous = self.entries.remove(index);
        }
        self.entries.push(Entry {
            identity: Arc::new(()),
            revision,
            attachment,
            visibility,
        });
        Ok(())
    }

    /// File/plugin/forge selections are user-owned and removed; source-owned
    /// browser, review and history entries remain suppressed in this composer.
    pub fn remove(&mut self, attachment: &ContextAttachment) {
        if source_owned(attachment) {
            for entry in &mut self.entries {
                if entry.attachment.same_key(attachment) {
                    entry.visibility = ContextVisibility::Suppressed;
                }
            }
        } else {
            self.entries
                .retain(|entry| !entry.attachment.same_key(attachment));
        }
    }

    /// Picker toggling removes a selected plugin/forge item or attaches it.
    ///
    /// # Errors
    /// Refuses invalid input or an unavailable transition with `ContextProblem`.
    pub fn toggle(&mut self, attachment: ContextAttachment) -> Result<(), ContextProblem> {
        if self.visible().any(|held| held.same_key(&attachment)) {
            self.remove(&attachment);
            Ok(())
        } else {
            self.reattach(attachment)
        }
    }

    /// An explicit producer action can show a previously suppressed item again.
    ///
    /// # Errors
    /// Refuses invalid input or an unavailable transition with `ContextProblem`.
    pub fn reattach(&mut self, attachment: ContextAttachment) -> Result<(), ContextProblem> {
        if let Some(index) = self
            .entries
            .iter()
            .position(|entry| entry.attachment.same_key(&attachment))
        {
            let revision = self.next()?;
            let _previous = self.entries.remove(index);
            self.entries.push(Entry {
                identity: Arc::new(()),
                revision,
                attachment,
                visibility: ContextVisibility::Visible,
            });
            Ok(())
        } else {
            self.add(attachment)
        }
    }

    /// Lock one immutable snapshot; another submit cannot replace it.
    ///
    /// # Errors
    /// Refuses invalid input or an unavailable transition with `ContextProblem`.
    pub fn prepare(&mut self) -> Result<Option<Submission>, ContextProblem> {
        if self.phase != Phase::Idle {
            return Err(ContextProblem::Busy);
        }
        let entries: Vec<Entry> = self
            .entries
            .iter()
            .filter(|entry| entry.visibility == ContextVisibility::Visible)
            .cloned()
            .collect();
        if entries.is_empty() {
            return Ok(None);
        }
        let total = entries.iter().try_fold(0usize, |total, entry| {
            total.checked_add(entry.attachment.text().len())
        });
        if total.is_none_or(|total| total > super::ContextText::MAX_BYTES) {
            return Err(ContextProblem::Text);
        }
        let token = self.next()?;
        self.phase = Phase::Submitting(token);
        Ok(Some(Submission { token, entries }))
    }

    /// Stale completions cannot clear a newer submission or newly updated data.
    #[must_use]
    pub fn settle(&mut self, submission: &Submission, outcome: SubmissionOutcome) -> bool {
        if self.phase != Phase::Submitting(submission.token) {
            return false;
        }
        self.phase = Phase::Idle;
        match outcome {
            SubmissionOutcome::Sent | SubmissionOutcome::Queued => self.entries.retain(|current| {
                !submission.entries.iter().any(|sent| {
                    sent.revision == current.revision
                        && sent.attachment.same_key(&current.attachment)
                })
            }),
            SubmissionOutcome::Failed => {
                for entry in &mut self.entries {
                    entry.visibility = ContextVisibility::Visible;
                }
            }
        }
        true
    }

    fn next(&mut self) -> Result<u64, ContextProblem> {
        self.clock = self.clock.checked_add(1).ok_or(ContextProblem::Exhausted)?;
        Ok(self.clock)
    }
}

fn replaceable(attachment: &ContextAttachment) -> bool {
    match attachment {
        ContextAttachment::WorkspaceFile(_) | ContextAttachment::Plugin(_) => false,
        ContextAttachment::Snapshot { kind, .. } => match kind {
            ContextKind::ChatHistory
            | ContextKind::PullComment
            | ContextKind::PullReview
            | ContextKind::PullCheck => true,
            ContextKind::BrowserElement
            | ContextKind::Review
            | ContextKind::ForgeIssue
            | ContextKind::ForgeChangeRequest => false,
        },
    }
}

fn source_owned(attachment: &ContextAttachment) -> bool {
    match attachment {
        ContextAttachment::WorkspaceFile(_) | ContextAttachment::Plugin(_) => false,
        ContextAttachment::Snapshot { kind, .. } => match kind {
            ContextKind::BrowserElement
            | ContextKind::Review
            | ContextKind::ChatHistory
            | ContextKind::PullComment
            | ContextKind::PullReview
            | ContextKind::PullCheck => true,
            ContextKind::ForgeIssue | ContextKind::ForgeChangeRequest => false,
        },
    }
}
