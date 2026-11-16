//! One agent per conversation file (protocol 17, ADR 0053): a structured
//! session that starts on an agent's existing conversation
//! (`session.create` with `resume`, an import, `session.resume`), or moves
//! to another profile, is refused while another live session holds the
//! same conversation's file, or is starting on it.
//!
//! [Decision Log]
//! - Purpose: two agents writing one transcript corrupt it (both append
//!   turns; each loads the other's half). Importing from the agent's own
//!   history makes resuming an id a click, so a double click, a stale
//!   `in_vessel` or a conversation a moved session still holds would start
//!   a second agent on it. Paseo refuses an active duplicate the same way.
//! - What "the same file" is: a conversation lives in a profile's home, so
//!   the same id under another profile is another file. The default
//!   profile is either `None` or the harness's name (a defined profile may
//!   not take it), so both are compared as `None`. Two operations put one
//!   conversation under several profiles:
//!   - `session.move_profile` (`moving`) hard-links the files into the
//!     target home, stops the agent and starts it there: the same inode is
//!     then under both profiles, for good.
//!   - The CLI's `vessel session move` and `copy` (protocol 6's transfers)
//!     export a copy and import it elsewhere: another file. The copy is
//!     resumed while the source still runs, by design, and that stays
//!     allowed.
//! - Choice: a session holds its conversation under every profile its
//!   files are linked into: the one it started under, and each profile a
//!   move took it to or from (`profile_changed`; for a session started
//!   again the store reads them from its log). A move claims its target
//!   before it links, and the session keeps it once linked, also when the
//!   new agent did not start (the links stay, `moving`). A start or a move
//!   is refused `conflict` when its profiles and those of a live session
//!   on the same conversation meet, or those of another start or move in
//!   flight. The claim is taken under a `std` mutex before the start and
//!   kept until the session is registered (then the live check sees it),
//!   so two starts at once are caught too.
//! - Alternatives: comparing the transcript's (device, inode) through
//!   `session_files` (exact, but the runtime would need every live
//!   session's home and folder at hand); a lock per conversation held
//!   across the start (the same thing with a lock held across `.await`).
//! - Cost: one pass over the live sessions per such start (a few hundred at
//!   most). A log pruned past its first move (retention, 64 MiB) forgets
//!   that move's profile.

use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex, PoisonError};

use vessel_harness::Profile;
use vessel_proto::{HarnessRef, HarnessSessionId, ProfileName, SessionId, SessionMode};

use super::SessionRuntime;
use crate::error::SessionError;

/// Profiles, each `None` for the harness's default.
pub(crate) type Profiles = BTreeSet<Option<ProfileName>>;

/// A conversation of an agent.
type Conversation = (HarnessRef, HarnessSessionId);

/// Starts and moves in flight, and the profiles each live session's files
/// are linked into.
#[derive(Debug, Default)]
pub(crate) struct Claims {
    /// Conversations being started on or moved, each with the session and
    /// the profiles it will hold.
    starting: Mutex<HashMap<Conversation, Vec<(SessionId, Profiles)>>>,
    /// What each registered session holds; a session absent here holds its
    /// current profile only.
    held: Mutex<HashMap<SessionId, Profiles>>,
}

/// A claim on a conversation, given back when dropped.
#[derive(Debug)]
pub(crate) struct Claim {
    claims: Arc<Claims>,
    conversation: Conversation,
    id: SessionId,
    profiles: Profiles,
}

impl Claim {
    /// The session started (or moved): it holds the claim's profiles from
    /// now on. The starting entry stays until the last shared owner drops,
    /// so a cancelled preparation cannot release it before physical settlement.
    pub(crate) fn keep(&self) {
        let _previous = self
            .claims
            .held
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(self.id, self.profiles.clone());
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        let mut starting = self
            .claims
            .starting
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(entries) = starting.get_mut(&self.conversation) {
            entries.retain(|(id, _)| *id != self.id);
            if entries.is_empty() {
                let _gone = starting.remove(&self.conversation);
            }
        }
    }
}

impl SessionRuntime {
    /// Claim `conversation` of `harness` for session `id`, which will hold
    /// it under `profiles` (each `None` for the default).
    ///
    /// # Errors
    /// [`SessionError::ConversationTaken`] when a live session or another
    /// start holds it under one of those profiles.
    pub(crate) fn claim(
        &self,
        id: SessionId,
        harness: &HarnessRef,
        profiles: Profiles,
        conversation: &HarnessSessionId,
    ) -> Result<Claim, SessionError> {
        let profiles: Profiles = profiles
            .into_iter()
            .map(|profile| canonical(harness, profile))
            .collect();
        let key = (harness.clone(), conversation.clone());
        let mut starting = self
            .claims
            .starting
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some((other, _)) = starting.get(&key).and_then(|entries| {
            entries
                .iter()
                .find(|(other, held)| *other != id && !held.is_disjoint(&profiles))
        }) {
            return Err(SessionError::ConversationTaken(*other));
        }
        // Under the claims' lock, so a start that registers its session
        // after this pass has its claim seen instead.
        let held = self
            .claims
            .held
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let live = self.list().into_iter().find(|info| {
            info.id != id
                && info.status.is_live()
                && info.spec.harness == *harness
                && on(info).is_some_and(|on| on == conversation)
                && held.get(&info.id).map_or_else(
                    || profiles.contains(&canonical(harness, info.profile.clone())),
                    |theirs| !theirs.is_disjoint(&profiles),
                )
        });
        if let Some(info) = live {
            return Err(SessionError::ConversationTaken(info.id));
        }
        starting
            .entry(key.clone())
            .or_default()
            .push((id, profiles.clone()));
        Ok(Claim {
            claims: Arc::clone(&self.claims),
            conversation: key,
            id,
            profiles,
        })
    }

    /// The profiles `id`'s files are linked into, as kept by its last
    /// start or move; its current one alone when none was kept.
    pub(crate) fn held(&self, id: SessionId, current: Option<&ProfileName>) -> Profiles {
        self.claims
            .held
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&id)
            .cloned()
            .unwrap_or_else(|| BTreeSet::from([current.cloned()]))
    }

    /// Forget what `id` held: it is removed from the runtime.
    pub(crate) fn release(&self, id: SessionId) {
        let _gone = self
            .claims
            .held
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&id);
    }
}

/// `profile` with the harness's default as `None`, however it was named.
fn canonical(harness: &HarnessRef, profile: Option<ProfileName>) -> Option<ProfileName> {
    let default = match harness {
        HarnessRef::Known(kind) => Profile::default_for(*kind).name().clone(),
        HarnessRef::Added(agent) => agent.profile().clone(),
    };
    profile.filter(|name| *name != default)
}

/// The conversation a session is on: the one its agent reported, or the
/// one it is starting on.
fn on(info: &vessel_proto::SessionInfo) -> Option<&HarnessSessionId> {
    info.harness_session.as_ref().or(match &info.spec.mode {
        SessionMode::Structured { resume, .. } => resume.as_ref(),
        SessionMode::Terminal { .. } => None,
    })
}
