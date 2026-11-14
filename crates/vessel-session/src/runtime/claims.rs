//! One agent per conversation (protocol 14, ADR 0053): a structured session
//! that starts on an agent's existing conversation (`session.create` with
//! `resume`, an import, `session.resume`) is refused while another live
//! session of this runtime is on it, or starting on it, under the same
//! profile.
//!
//! [Decision Log]
//! - Purpose: two agents writing one transcript corrupt it (both append
//!   turns; each loads the other's half). Importing from the agent's own
//!   history makes resuming an id a click, so a double click, a stale
//!   `in_vessel` or a conversation a moved session still holds would start
//!   a second agent on it. Paseo refuses an active duplicate the same way.
//! - Alternatives: checking at the node before `create` (two starts at once
//!   both pass: the session is registered only once its agent is spawned);
//!   a lock per conversation held across the start (the check below is the
//!   same thing without a lock held across `.await`).
//! - Choice: a claim, taken under a `std` mutex before the start and kept
//!   until the session is registered (then the live check sees it), so a
//!   second start on the conversation, at once or later, is refused
//!   `conflict` with the session that has it. An ended session holds
//!   nothing: its agent is gone.
//! - Per profile: the transcript is in the profile's home, so the same id
//!   under another profile is another file (`session move` and `session
//!   copy` resume a copy there while the source still runs, by design).
//!   A move between profiles links the files instead, so the profile a
//!   session moved away from still holds its transcript: a deliberate
//!   import there is not caught (its listing names the session in
//!   `in_vessel`).
//! - Cost: one pass over the live sessions per such start (a few hundred
//!   at most).

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use vessel_proto::{HarnessRef, HarnessSessionId, ProfileName, SessionId, SessionMode};

use super::SessionRuntime;
use crate::error::SessionError;

/// A conversation of an agent, under a profile (`None`: the default).
type Conversation = (HarnessRef, Option<ProfileName>, HarnessSessionId);

/// Conversations sessions are starting on, each with the session.
#[derive(Debug, Default)]
pub(crate) struct Claims(Mutex<HashMap<Conversation, SessionId>>);

/// A claim on a conversation, given back when dropped.
#[derive(Debug)]
pub(crate) struct Claim<'a> {
    claims: &'a Claims,
    conversation: Conversation,
}

impl Drop for Claim<'_> {
    fn drop(&mut self) {
        let _released = self
            .claims
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&self.conversation);
    }
}

impl SessionRuntime {
    /// Claim `conversation` of `harness` under `profile` (`None`: the
    /// harness's default) for session `id` starting on it.
    ///
    /// # Errors
    /// [`SessionError::ConversationTaken`] when a live session is on it or
    /// another is starting on it.
    pub(crate) fn claim(
        &self,
        id: SessionId,
        harness: &HarnessRef,
        profile: Option<&ProfileName>,
        conversation: &HarnessSessionId,
    ) -> Result<Claim<'_>, SessionError> {
        let key = (harness.clone(), profile.cloned(), conversation.clone());
        let mut claims = self.claims.0.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(other) = claims.get(&key) {
            return Err(SessionError::ConversationTaken(*other));
        }
        // Under the claims' lock, so a start that registers its session
        // after this pass has its claim seen instead.
        let live = self.list().into_iter().find(|info| {
            info.id != id
                && info.status.is_live()
                && info.spec.harness == *harness
                && info.profile.as_ref() == profile
                && on(info).is_some_and(|on| on == conversation)
        });
        if let Some(info) = live {
            return Err(SessionError::ConversationTaken(info.id));
        }
        let _none = claims.insert(key.clone(), id);
        Ok(Claim {
            claims: &self.claims,
            conversation: key,
        })
    }
}

/// The conversation a session is on: the one its agent reported, or the
/// one it is starting on.
fn on(info: &vessel_proto::SessionInfo) -> Option<&HarnessSessionId> {
    info.harness_session.as_ref().or(match &info.spec.mode {
        SessionMode::Structured { resume, .. } => resume.as_ref(),
        SessionMode::Terminal { .. } => None,
    })
}
