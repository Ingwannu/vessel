//! The pictures of sent images (track P's row c, its third step): a
//! message that carried an image shows the image in the conversation, read
//! back from its node (`attachment.read`) when its row is drawn, or kept
//! from the composer when it was sent from this app.
//!
//! [Decision Log]
//! - Purpose: Paseo's user message shows each image it carried
//!   (`components/message.tsx`, `UserMessageImagePill`); so does the page,
//!   for images sent from any device.
//! - Constraint: the conversation is drawn a screen at a time, and a
//!   session may carry a hundred megabytes of files; reading every image
//!   when a session opens would read all of them. The node hands a file
//!   out a mebibyte at a time.
//! - Followed: the page's rules (`web/src/data/images.ts`): a read when a
//!   row asks, two at a time, one read per image however many rows ask; at
//!   most 48 pictures or 64 MiB kept, the least recently asked dropped
//!   first; an image sent from here kept from its bytes, never read back.
//! - Choice: the rules here, without a toolkit, so the desktop and the
//!   phone decide alike; a failed read is not tried again while it is kept
//!   (the row shows the frame without its picture).
//! - Cost: an image dropped from the store and scrolled back to is read
//!   again.

use std::collections::HashMap;
use std::sync::Arc;

use vessel_proto::{AttachmentId, AttachmentKind, AttachmentLimits, AttachmentRef, SessionId};

/// Which sent image: its machine, its session and its id there.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Sent {
    /// Welcome identity captured before the read; aliases cannot replace it.
    pub node: vessel_proto::NodeId,
    /// The machine the session is on.
    pub machine: crate::machine_book::Name,
    /// The session.
    pub session: SessionId,
    /// The image.
    pub attachment: AttachmentId,
}

/// A sent image's picture, as the store keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Picture {
    /// Asked for; its read waits its turn.
    Wanted,
    /// Being read.
    Reading,
    /// Its bytes, as the image file holds them.
    Ready(Arc<[u8]>),
    /// It could not be read.
    Failed,
}

/// The sent images' pictures, each with when it was last asked for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pictures {
    kept: HashMap<Sent, (u64, Picture)>,
    clock: u64,
}

impl Pictures {
    /// The most pictures kept (the page's).
    pub const MOST: usize = 48;
    /// The most bytes kept (the page's).
    pub const MOST_BYTES: u64 = 64 * 1024 * 1024;
    /// The most reads at once (the page's).
    pub const AT_ONCE: usize = 2;

    /// Whether `attachment` is shown as a picture: an image no larger than
    /// a node takes one (a larger one cannot have been sent).
    #[must_use]
    pub fn shows(attachment: &AttachmentRef) -> bool {
        attachment.kind == AttachmentKind::Image
            && attachment.size <= AttachmentLimits::DEFAULT.image
    }

    /// `sent`'s picture, asked for now: the most recently asked; one not
    /// kept is wanted, and read in its turn ([`Self::starts`]).
    pub fn ask(&mut self, sent: &Sent) -> Picture {
        self.clock = self.clock.saturating_add(1);
        let clock = self.clock;
        let (used, picture) = self
            .kept
            .entry(sent.clone())
            .or_insert((clock, Picture::Wanted));
        *used = clock;
        let asked = picture.clone();
        self.bound();
        asked
    }

    /// The reads to begin now, at most [`Self::AT_ONCE`] running, the most
    /// recently asked first: each is marked as being read.
    pub fn starts(&mut self) -> Vec<Sent> {
        let running = self
            .kept
            .values()
            .filter(|(_, picture)| *picture == Picture::Reading)
            .count();
        let room = Self::AT_ONCE.saturating_sub(running);
        let mut wanted: Vec<(u64, Sent)> = self
            .kept
            .iter()
            .filter(|(_, (_, picture))| *picture == Picture::Wanted)
            .map(|(sent, (used, _))| (*used, sent.clone()))
            .collect();
        wanted.sort_by_key(|item| std::cmp::Reverse(item.0));
        wanted.truncate(room);
        for (_, sent) in &wanted {
            if let Some((_, picture)) = self.kept.get_mut(sent) {
                *picture = Picture::Reading;
            }
        }
        wanted.into_iter().map(|(_, sent)| sent).collect()
    }

    /// `sent`'s read ended: its bytes, or `None` when it failed. A picture
    /// dropped meanwhile takes nothing.
    pub fn read(&mut self, sent: &Sent, bytes: Option<Arc<[u8]>>) {
        if let Some((_, picture)) = self.kept.get_mut(sent) {
            *picture = bytes.map_or(Picture::Failed, Picture::Ready);
        }
        self.bound();
    }

    /// An image sent from this app, kept from its bytes: never read back.
    pub fn seed(&mut self, sent: Sent, bytes: Arc<[u8]>) {
        self.clock = self.clock.saturating_add(1);
        drop(self.kept.insert(sent, (self.clock, Picture::Ready(bytes))));
        self.bound();
    }

    /// How many pictures are kept.
    #[must_use]
    pub fn len(&self) -> usize {
        self.kept.len()
    }

    /// Whether none is.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.kept.is_empty()
    }

    /// The bytes kept.
    #[must_use]
    pub fn bytes(&self) -> u64 {
        self.kept
            .values()
            .map(|(_, picture)| match picture {
                Picture::Ready(bytes) => u64::try_from(bytes.len()).unwrap_or(u64::MAX),
                Picture::Wanted | Picture::Reading | Picture::Failed => 0,
            })
            .sum()
    }

    /// Drops the least recently asked pictures while more than
    /// [`Self::MOST`] or [`Self::MOST_BYTES`] are kept. One being read
    /// stays (its read is counted against [`Self::AT_ONCE`] until it
    /// ends).
    fn bound(&mut self) {
        while self.kept.len() > Self::MOST || self.bytes() > Self::MOST_BYTES {
            let oldest = self
                .kept
                .iter()
                .filter(|(_, (_, picture))| *picture != Picture::Reading)
                .min_by_key(|(_, (used, _))| *used)
                .map(|(sent, _)| sent.clone());
            let Some(oldest) = oldest else {
                return;
            };
            drop(self.kept.remove(&oldest));
        }
    }
}

#[cfg(test)]
mod tests;
