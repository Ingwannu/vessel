//! Host Voice's node-bound read/write presentation, without a settings store.
//!
//! The canonical DTOs own provider/default/reset policy. This page only keeps
//! actual replies, excludes stale callbacks and requests one switch change.
//! A missing or failed read never becomes a default configuration.

use crate::devices::Ticket;
use vessel_proto::dictation::{
    DictationChange, DictationProvider, DictationSettings, DictationSwitch, SetDictation,
};
use vessel_proto::voice::{SetVoice, VoiceSettings};
use vessel_proto::{ErrorCode, NodeId};

/// Three independent settings editors, not three independent enabled flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Site {
    /// The composer's speech recognition provider.
    Dictation,
    /// Conversation recognition, independent of dictation.
    Recognition,
    /// Conversation synthesis, independent of both recognition sites.
    Synthesis,
}

/// The two independent feature switches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    /// Dictation's own default is On.
    Dictation,
    /// Conversation's own default is Off.
    Conversation,
}

/// Presence-only settings from one successfully read node.
#[derive(Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// Canonical dictation metadata; no credential value.
    pub dictation: DictationSettings,
    /// Canonical conversation metadata; no credential value.
    pub conversation: VoiceSettings,
}
impl std::fmt::Debug for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpeechSettingsSnapshot")
            .finish_non_exhaustive()
    }
}

/// Static typed failures; foreign error text never enters the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    /// The machine is disconnected.
    Offline,
    /// This host does not support reading these settings.
    Unsupported,
    /// A failed call is distinct from an absent setting.
    Refused(ErrorCode),
}

/// A canonical change offered to an adapter, not a fabricated node RPC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// Only dictation's switch is changed.
    Dictation(SetDictation),
    /// Only conversation's switch is changed.
    Conversation(SetVoice),
}

/// A request remains bound to its original machine and operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Write {
    /// Exact destination, not the current book selection.
    pub node: NodeId,
    /// The owning page checks it again when the reply arrives.
    pub ticket: Ticket,
    /// Existing canonical values, without a speculative `NodeSettings` field.
    pub change: Change,
}

/// A switch callback captured from one displayed snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Toggle {
    node: NodeId,
    source: Ticket,
    feature: Feature,
}

/// Opening an editor is also bound to the displayed host and snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edit {
    node: NodeId,
    source: Ticket,
    site: Site,
}

/// Whether the current snapshot can accept another edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Editing {
    /// No write is outstanding.
    Idle,
    /// A write must settle before another mutation.
    Saving,
}

/// What the unregistered page draws, borrowing the actual successful snapshot.
#[derive(Debug)]
pub enum Status<'a> {
    /// No result yet; do not draw default switches.
    Loading,
    /// No usable snapshot was obtained.
    Failed(Problem),
    /// A failed save preserves the last confirmed values.
    Ready(&'a Snapshot, Editing, Option<Problem>),
}

/// One selected node's page, independent of Shell, transport and persistence.
#[derive(Debug)]
pub struct Page {
    node: NodeId,
    ticket: Ticket,
    read: Option<Result<Snapshot, Problem>>,
    pending: Pending,
    failure: Option<Problem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pending {
    Read,
    Write,
    Settled,
}

impl Page {
    /// Start a fresh read. Replacing a page does not reuse its operation ticket.
    #[must_use]
    pub const fn reading(node: NodeId, ticket: Ticket) -> Self {
        Self {
            node,
            ticket,
            read: None,
            pending: Pending::Read,
            failure: None,
        }
    }

    /// The original host remains the destination through a pending save.
    #[must_use]
    pub const fn node(&self) -> NodeId {
        self.node
    }

    /// Loading, failure and successful values stay distinguishable.
    #[must_use]
    pub fn status(&self) -> Status<'_> {
        match &self.read {
            None => Status::Loading,
            Some(Err(problem)) => Status::Failed(*problem),
            Some(Ok(snapshot)) => Status::Ready(
                snapshot,
                if self.pending == Pending::Write {
                    Editing::Saving
                } else {
                    Editing::Idle
                },
                self.failure,
            ),
        }
    }

    /// Reject a retained callback for another host or an older operation.
    /// A save refusal preserves the successful snapshot rather than inventing rollback.
    pub fn answered(
        &mut self,
        node: NodeId,
        ticket: Ticket,
        reply: Result<Snapshot, Problem>,
    ) -> super::Applied {
        if node != self.node || ticket != self.ticket || self.pending == Pending::Settled {
            return super::Applied::Stale;
        }
        if self.pending == Pending::Write {
            match reply {
                Ok(snapshot) => {
                    self.read = Some(Ok(snapshot));
                    self.failure = None;
                }
                Err(problem) => self.failure = Some(problem),
            }
        } else {
            self.read = Some(reply);
            self.failure = None;
        }
        self.pending = Pending::Settled;
        super::Applied::Current
    }

    /// Capture an action only for known switches in the current editable snapshot.
    #[must_use]
    pub fn toggle_action(&self, feature: Feature) -> Option<Toggle> {
        if !self.writable() {
            return None;
        }
        let snapshot = self.read.as_ref()?.as_ref().ok()?;
        let current = match feature {
            Feature::Dictation => snapshot.dictation.switch,
            Feature::Conversation => snapshot.conversation.switch,
        };
        if current == DictationSwitch::Unknown {
            return None;
        }
        Some(Toggle {
            node: self.node,
            source: self.ticket,
            feature,
        })
    }

    /// Recheck a retained callback before producing the one requested switch patch.
    /// Sibling provider options are omitted; stale and duplicate actions do nothing.
    pub fn toggle(&mut self, action: Toggle, ticket: Ticket) -> Option<Write> {
        if self.toggle_action(action.feature) != Some(action) || ticket == self.ticket {
            return None;
        }
        let snapshot = self.read.as_ref()?.as_ref().ok()?;
        let current = match action.feature {
            Feature::Dictation => snapshot.dictation.switch,
            Feature::Conversation => snapshot.conversation.switch,
        };
        let next = match current {
            DictationSwitch::On => DictationSwitch::Off,
            DictationSwitch::Off => DictationSwitch::On,
            DictationSwitch::Unknown => return None,
        };
        let change = match action.feature {
            Feature::Dictation => Change::Dictation(SetDictation {
                switch: Some(DictationChange::Set(next)),
                ..SetDictation::default()
            }),
            Feature::Conversation => Change::Conversation(SetVoice {
                switch: Some(DictationChange::Set(next)),
                ..SetVoice::default()
            }),
        };
        self.ticket = ticket;
        self.pending = Pending::Write;
        self.failure = None;
        Some(Write {
            node: self.node,
            ticket,
            change,
        })
    }

    /// Unknown providers are readable, but cannot open a known-provider editor.
    #[must_use]
    pub fn can_edit(&self, site: Site) -> bool {
        if !self.writable() {
            return false;
        }
        let Status::Ready(snapshot, Editing::Idle, _) = self.status() else {
            return false;
        };
        let provider = match site {
            Site::Dictation => snapshot.dictation.provider,
            Site::Recognition => snapshot.conversation.stt.provider,
            Site::Synthesis => snapshot.conversation.tts.provider,
        };
        provider != DictationProvider::Unknown
    }

    /// Capture a known site's edit action for this exact displayed snapshot.
    #[must_use]
    pub fn edit_action(&self, site: Site) -> Option<Edit> {
        self.can_edit(site).then_some(Edit {
            node: self.node,
            source: self.ticket,
            site,
        })
    }

    /// A retained row cannot open an editor on another host or after a save.
    /// The editor owner allocates its fresh sheet lease only after this check.
    #[must_use]
    pub fn edit(&self, action: Edit) -> Option<Site> {
        (self.edit_action(action.site) == Some(action)).then_some(action.site)
    }

    fn writable(&self) -> bool {
        self.pending == Pending::Settled
            && !matches!(
                self.failure,
                Some(
                    Problem::Offline
                        | Problem::Unsupported
                        | Problem::Refused(ErrorCode::Forbidden)
                )
            )
    }
}

#[cfg(test)]
mod tests;
