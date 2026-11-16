//! Fullscreen, locking and per-mode affordances from the reference composer.

/// A composer has the same editor in either use, with different affordances.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Mode {
    /// Agent input offers context and commands.
    #[default]
    Chat,
    /// Terminal input becomes argv and has no agent affordances.
    Terminal,
}

/// Whether a host permits edits independently of submission state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Access {
    /// The owner can edit.
    #[default]
    Editable,
    /// Read-only clients retain their text but cannot change it.
    ReadOnly,
}

/// How an owner handles the draft during a submission.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SubmitPolicy {
    /// Clear immediately, restoring the draft on failure.
    #[default]
    Clear,
    /// Creation forms retain their input and lock it until a result arrives.
    PreserveAndLock,
}

/// Submission progress affects editing only under preserve-and-lock.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Submission {
    /// No operation awaits an answer.
    #[default]
    Idle,
    /// Attachments are being prepared.
    Preparing,
    /// A correlated send awaits the node.
    Sending,
    /// The send was accepted.
    Accepted,
    /// Failure restores the draft and permits another attempt.
    Rejected,
}

/// Whether another composer surface currently covers the editor.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Overlay {
    /// The text owns its slot.
    #[default]
    None,
    /// Dictation owns its slot.
    Dictation,
    /// Realtime voice owns its slot.
    Voice,
}

/// The same editor either stays in the dock or fills its pane.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Extent {
    /// Inline dock.
    #[default]
    Inline,
    /// Expanded compact editor.
    Fullscreen,
}

/// Layout classification is provided by the host's existing breakpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// Pointer layout.
    Desktop,
    /// Touch layout.
    Compact,
}

/// A measured rendered line count, distinct from explicit newline count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderedLines(usize);

impl RenderedLines {
    /// A layout engine's count, with an empty field still occupying one line.
    #[must_use]
    pub const fn new(count: usize) -> Self {
        Self(if count == 0 { 1 } else { count })
    }
}

/// Reference threshold: the expansion control appears above two rendered lines.
const FULLSCREEN_LINE_THRESHOLD: usize = 2;

/// Every input needed to decide whether a surface can be expanded.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Surface {
    /// Chat or terminal controls.
    mode: Mode,
    /// Owner permissions.
    access: Access,
    /// Draft retention semantics.
    submit_policy: SubmitPolicy,
    /// Progress of the current submit.
    submission: Submission,
    /// A voice surface can replace the editor.
    overlay: Overlay,
    /// Where the editor is mounted.
    extent: Extent,
}

impl Surface {
    /// Each host supplies its own immutable mode and submission policy.
    #[must_use]
    pub const fn new(mode: Mode, access: Access, submit_policy: SubmitPolicy) -> Self {
        Self {
            mode,
            access,
            submit_policy,
            submission: Submission::Idle,
            overlay: Overlay::None,
            extent: Extent::Inline,
        }
    }

    /// Terminal hosts suppress the chat-only controls using this single policy.
    #[must_use]
    pub const fn mode(self) -> Mode {
        self.mode
    }

    /// Project the retained owner's mode without replacing its editor or state.
    /// Admission and any audio lease transition remain with the host owner.
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    /// The presentation is read through the controller, never mutated by a screen.
    #[must_use]
    pub const fn extent(self) -> Extent {
        self.extent
    }

    /// Resource transfer waits until an audio surface releases the editor.
    #[must_use]
    pub const fn overlay(self) -> Overlay {
        self.overlay
    }

    /// A submit leaves fullscreen before any preparation or network wait begins.
    pub fn set_submission(&mut self, submission: Submission) {
        self.submission = submission;
        match submission {
            Submission::Preparing | Submission::Sending => self.collapse(),
            Submission::Idle | Submission::Accepted | Submission::Rejected => {}
        }
    }

    /// Preserve-and-lock must stop edits before a create operation finishes.
    #[must_use]
    pub const fn access(self) -> Access {
        let loading = match self.submission {
            Submission::Preparing | Submission::Sending => true,
            Submission::Idle | Submission::Accepted | Submission::Rejected => false,
        };
        if matches!(self.access, Access::ReadOnly)
            || (matches!(self.submit_policy, SubmitPolicy::PreserveAndLock) && loading)
        {
            Access::ReadOnly
        } else {
            Access::Editable
        }
    }

    /// A chat menu cannot cover terminal input, a voice surface or fullscreen text.
    #[must_use]
    pub fn autocomplete(self) -> bool {
        self.mode == Mode::Chat
            && self.extent == Extent::Inline
            && self.overlay == Overlay::None
            && self.access() == Access::Editable
    }

    /// Only compact editable surfaces with multiple rendered lines expand.
    #[must_use]
    pub fn can_expand(self, layout: Layout, lines: RenderedLines) -> bool {
        layout == Layout::Compact
            && self.access() == Access::Editable
            && self.overlay == Overlay::None
            && lines.0 > FULLSCREEN_LINE_THRESHOLD
    }

    /// Expand only after the same conditions that reveal its control hold.
    pub fn expand(&mut self, layout: Layout, lines: RenderedLines) {
        if self.can_expand(layout, lines) {
            self.extent = Extent::Fullscreen;
        }
    }

    /// A covering voice surface ends expansion, retaining the same text owner.
    pub fn cover(&mut self, overlay: Overlay) {
        self.overlay = overlay;
        if overlay != Overlay::None {
            self.extent = Extent::Inline;
        }
    }

    /// Escape/back or a send returns the editor to its dock.
    pub fn collapse(&mut self) {
        self.extent = Extent::Inline;
    }

    /// Read-only changes cannot leave an editable expanded surface behind.
    pub fn set_access(&mut self, access: Access) {
        self.access = access;
        if self.access() == Access::ReadOnly {
            self.collapse();
        }
    }
}

#[cfg(test)]
mod tests;
