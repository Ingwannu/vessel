//! The files attached to one composer and their uploads (track P's row
//! c): picked, pasted or dropped, checked against what the agent takes and
//! the node's limits as they come, uploaded when the prompt is sent, each
//! with its progress, failure, retry and removal.
//!
//! [Decision Log]
//! - Purpose: Paseo attaches files to the next message, shows each as a
//!   pill over the text, and sends them with it
//!   (`composer/index.tsx`, `renderAttachmentTray`).
//! - Constraint: Vessel's node keeps attachments with the session (ADR
//!   0025), so a file must be uploaded, in pieces, before the prompt that
//!   names it; an upload needs a live session whose agent has said what it
//!   takes (images only after its handshake), and the limits come from the
//!   conversation. The node has no call to give an upload back: whatever
//!   is begun counts against the session's room until the session goes.
//! - Followed: Vessel's own page (`web/src/data/tray.ts`,
//!   `tray-item.ts`), which settled the same constraints.
//! - Alternatives: uploading as soon as a file is attached (shows progress
//!   early, but spends the session's room on files the person removes);
//!   keeping the bytes in the tray (a 50 MiB file in memory per pill).
//! - Choice: a file is checked when it is attached (or, with no session
//!   yet, once there is one) and uploaded when Send is pressed, at most two
//!   at a time; the prompt goes when every file is in. A refused file, or
//!   one whose upload failed, stops Send until it is removed or retried; a
//!   retry goes on from what the node has. The tray holds where the bytes
//!   are ([`Origin`]: a file on this device, read a piece at a time when it
//!   is sent, or an image pasted, held until it is sent or removed).
//! - Cost: Send waits for the uploads.

use std::path::PathBuf;
use std::sync::Arc;

use vessel_proto::conversation::MimeType;
use vessel_proto::{
    AgentAbilities, AttachmentKind, AttachmentLimits, AttachmentName, AttachmentRef, UploadId,
};

use super::attach::{Candidate, Refusal, attachment_name, kind_of, mime_of, refusal};

/// Files uploading at once from one composer (the node allows a caller
/// four; two leaves room for another window, as the page does).
pub const AT_ONCE: usize = 2;

/// A file's place in one tray, for its pill and its upload's answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemKey(u64);

/// Where a file's bytes are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// A file on this device, read a piece at a time as it is sent.
    File(PathBuf),
    /// An image pasted from the clipboard, held until it is sent.
    Pasted(Arc<[u8]>),
}

/// An upload the node holds, and how far it got (a retry goes on from
/// there rather than counting the file twice against the session's room).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Begun {
    /// The node's upload.
    pub upload: UploadId,
    /// The largest piece it takes.
    pub chunk: u32,
    /// The bytes it has.
    pub received: u64,
}

/// What became of a failed upload, for its retry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Again {
    /// Tried again from what the node has.
    Resume(Begun),
    /// Begun again (the node dropped the upload, or it never began).
    Restart,
    /// Not tried again (the file itself is wrong): only removed.
    Never,
}

/// Why an upload failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// An image whose first bytes are not its type's (the node refuses
    /// it at `attachment.finish`).
    NotItsType,
    /// The file could not be read on this device: the system's words.
    Unreadable(String),
    /// The file grew shorter while it was sent.
    Changed,
    /// The node refused, or the connection failed: its words.
    Node(String),
}

/// Where a file is on its way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemState {
    /// Not checked yet: no session, or its agent has not said what it
    /// takes.
    Held,
    /// Refused before a byte was sent.
    Refused(Refusal),
    /// Checked; waiting for Send.
    Attached,
    /// Send was pressed; waiting for a place among [`AT_ONCE`].
    Waiting(Option<Begun>),
    /// Being sent: the bytes the node has.
    Uploading(u64),
    /// In: the reference the prompt names it by.
    Uploaded(AttachmentRef),
    /// Its upload failed.
    Failed {
        /// Why.
        problem: Problem,
        /// Whether, and how, it may be tried again.
        again: Again,
    },
}

/// One attached file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// Its place in the tray.
    pub key: ItemKey,
    /// Its name as the node will keep it.
    pub name: AttachmentName,
    /// Its type, from its name.
    pub mime: MimeType,
    /// What it is to the agent.
    pub kind: AttachmentKind,
    /// Its size in bytes.
    pub size: u64,
    /// Where its bytes are.
    pub origin: Origin,
    /// Where it is on its way.
    pub state: ItemState,
}

/// What the files are checked against: the session's agent and the node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    /// What the agent takes, once its handshake said.
    pub abilities: Option<AgentAbilities>,
    /// The node's limits.
    pub limits: AttachmentLimits,
}

/// Whether a tray's files can go with a prompt now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Readiness {
    /// Every file is in: the references, in the tray's order.
    Ready(Vec<AttachmentRef>),
    /// Some are still on their way.
    Uploading,
    /// A file was refused or failed: it must be removed (or retried).
    Blocked(ItemKey),
}

/// One composer's attached files.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tray {
    items: Vec<Item>,
    next: u64,
    target: Option<Target>,
}

impl Tray {
    /// The files, in the order they were attached.
    #[must_use]
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// Whether nothing is attached.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Attach the file named `name`, of `size` bytes, from `origin`
    /// (checked at once when there is a target); its key, or `None` when
    /// no name could be made of it.
    pub fn add(&mut self, name: &str, size: u64, origin: Origin) -> Option<ItemKey> {
        let fallback = if matches!(origin, Origin::Pasted(_)) {
            "image.png"
        } else {
            "file"
        };
        let name = attachment_name(name, fallback)?;
        let mime = mime_of(name.as_str())?;
        let key = ItemKey(self.next);
        self.next = self.next.saturating_add(1);
        self.items.push(Item {
            key,
            kind: kind_of(&mime),
            name,
            mime,
            size,
            origin,
            state: ItemState::Held,
        });
        self.settle();
        Some(key)
    }

    /// Take a file out (its upload, if one runs, is the caller's to stop).
    /// A file refused for the count or the total is looked at again: the
    /// room it lacked may be there now (the page keeps it refused until the
    /// session changes).
    pub fn remove(&mut self, key: ItemKey) -> bool {
        let before = self.items.len();
        self.items.retain(|item| item.key != key);
        for item in &mut self.items {
            if matches!(item.state, ItemState::Refused(_)) {
                item.state = ItemState::Held;
            }
        }
        self.settle();
        self.items.len() != before
    }

    /// What the files are checked against from now; `None` while there is
    /// no session. A refused file is looked at again (the agent may take
    /// now what it did not).
    pub fn target(&mut self, target: Option<Target>) {
        if self.target == target {
            return;
        }
        self.target = target;
        if target.is_some() {
            for item in &mut self.items {
                if matches!(item.state, ItemState::Refused(_)) {
                    item.state = ItemState::Held;
                }
            }
        }
        self.settle();
    }

    /// Send was pressed: every checked file waits for its upload.
    pub fn send(&mut self) {
        for item in &mut self.items {
            if item.state == ItemState::Attached {
                item.state = ItemState::Waiting(None);
            }
        }
    }

    /// The uploads to start now, at most [`AT_ONCE`] running: each is
    /// marked uploading and handed back with where to go on from.
    pub fn starts(&mut self) -> Vec<(Item, Option<Begun>)> {
        let running = self
            .items
            .iter()
            .filter(|item| matches!(item.state, ItemState::Uploading(_)))
            .count();
        let mut room = AT_ONCE.saturating_sub(running);
        let mut started = Vec::new();
        for item in &mut self.items {
            if room == 0 {
                break;
            }
            if let ItemState::Waiting(resume) = item.state {
                item.state = ItemState::Uploading(resume.map_or(0, |begun| begun.received));
                started.push((item.clone(), resume));
                room = room.saturating_sub(1);
            }
        }
        started
    }

    /// The node has `received` bytes of `key`'s file.
    pub fn progress(&mut self, key: ItemKey, received: u64) {
        if let Some(item) = self.item_mut(key)
            && matches!(item.state, ItemState::Uploading(_))
        {
            item.state = ItemState::Uploading(received);
        }
    }

    /// `key`'s file is in.
    pub fn uploaded(&mut self, key: ItemKey, attachment: AttachmentRef) {
        if let Some(item) = self.item_mut(key) {
            item.state = ItemState::Uploaded(attachment);
        }
    }

    /// `key`'s upload failed.
    pub fn failed(&mut self, key: ItemKey, problem: Problem, again: Again) {
        if let Some(item) = self.item_mut(key) {
            item.state = ItemState::Failed { problem, again };
        }
    }

    /// Try `key`'s failed upload again (it waits for a place).
    pub fn retry(&mut self, key: ItemKey) -> bool {
        let Some(item) = self.item_mut(key) else {
            return false;
        };
        let ItemState::Failed { again, .. } = &item.state else {
            return false;
        };
        let resume = match *again {
            Again::Resume(begun) => Some(begun),
            Again::Restart => None,
            Again::Never => return false,
        };
        item.state = ItemState::Waiting(resume);
        true
    }

    /// Whether the files can go with a prompt now.
    #[must_use]
    pub fn readiness(&self) -> Readiness {
        let mut references = Vec::with_capacity(self.items.len());
        let mut uploading = false;
        for item in &self.items {
            match &item.state {
                ItemState::Uploaded(attachment) => references.push(attachment.clone()),
                ItemState::Refused(_) | ItemState::Failed { .. } => {
                    return Readiness::Blocked(item.key);
                }
                ItemState::Held
                | ItemState::Attached
                | ItemState::Waiting(_)
                | ItemState::Uploading(_) => uploading = true,
            }
        }
        if uploading {
            Readiness::Uploading
        } else {
            Readiness::Ready(references)
        }
    }

    /// Empty the tray once its files went with a prompt.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    fn item_mut(&mut self, key: ItemKey) -> Option<&mut Item> {
        self.items.iter_mut().find(|item| item.key == key)
    }

    /// Check what is held against the target, in the order attached.
    fn settle(&mut self) {
        let Some(target) = self.target else {
            return;
        };
        let mut kept: Vec<Candidate> = Vec::with_capacity(self.items.len());
        for item in &mut self.items {
            let candidate = Candidate {
                kind: item.kind,
                size: item.size,
            };
            match item.state {
                ItemState::Refused(_) => {}
                ItemState::Held => {
                    let refused =
                        refusal(candidate, &kept, target.abilities.as_ref(), &target.limits);
                    if let Some(refused) = refused {
                        item.state = ItemState::Refused(refused);
                    } else {
                        kept.push(candidate);
                        item.state = ItemState::Attached;
                    }
                }
                ItemState::Attached
                | ItemState::Waiting(_)
                | ItemState::Uploading(_)
                | ItemState::Uploaded(_)
                | ItemState::Failed { .. } => kept.push(candidate),
            }
        }
    }
}

#[cfg(test)]
mod tests;
