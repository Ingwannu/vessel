//! Request correlation for git menus. A root alone cannot identify a
//! reply: closing A, opening B, then A again can leave A's first read in
//! flight. Keys are issued by the area's lifetime-long counter, never by
//! a dialog or a repository which can be recreated.
//!
//! [Decision Log]
//! - Purpose: late reads and completions must not change another menu.
//! - Constraint: an async reply can outlive its session's screen.
//! - Alternatives: match only the root, or cancel the network task.
//! - Choice: app-side monotonic keys, separate from disposable state.
//! - Why: a root may reopen, and cancellation cannot undo a git write.
//! - Cost: exhausted keys refuse new actions instead of wrapping; the
//!   link must echo the key without changing the node's wire.

use super::super::problem::Problem;

/// One app-side git request; never sent over the node wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestKey(u64);

/// The area's request-key issuer, kept across session and root changes.
#[derive(Debug, Default)]
pub struct RequestKeys(u64);

/// No further request can be identified without reusing an old key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("git request identifiers exhausted")]
pub struct KeysExhausted;

impl RequestKeys {
    /// Allocate a key, refusing rather than wrapping at exhaustion.
    ///
    /// # Errors
    /// [`KeysExhausted`] when every identifier has been issued.
    pub fn issue(&mut self) -> Result<RequestKey, KeysExhausted> {
        self.0 = self.0.checked_add(1).ok_or(KeysExhausted)?;
        Ok(RequestKey(self.0))
    }
}

/// A read's current state. Old content survives a refresh, but a failed
/// read remains visibly failed rather than making old facts look fresh.
#[derive(Debug, Default)]
pub enum Reading<T> {
    /// Nothing asked yet.
    #[default]
    Empty,
    /// Only this key may replace the prior content.
    Loading {
        /// The newest request.
        key: RequestKey,
        /// Content to keep in place during a refresh.
        previous: Option<T>,
    },
    /// The newest request answered.
    Ready(T),
    /// The newest request failed; old content is not actionable.
    Failed(Problem),
}

impl<T> Reading<T> {
    /// Start a newer read, retaining content during loading only.
    pub fn start(&mut self, key: RequestKey) {
        let previous = match std::mem::take(self) {
            Self::Ready(value) => Some(value),
            Self::Loading { previous, .. } => previous,
            Self::Empty | Self::Failed(_) => None,
        };
        *self = Self::Loading { key, previous };
    }

    /// Accept only the newest outstanding read. Returns whether the
    /// screen should apply the result; a late or duplicate reply is inert.
    pub fn accept(&mut self, key: RequestKey, result: Result<T, Problem>) -> bool {
        if !matches!(self, Self::Loading { key: pending, .. } if *pending == key) {
            return false;
        }
        *self = match result {
            Ok(value) => Self::Ready(value),
            Err(problem) => Self::Failed(problem),
        };
        true
    }

    /// The outstanding request's key, when one is out. The node's workspace
    /// answers echo the root they were asked about, not an app key: the
    /// caller matches that root first, then accepts with this key.
    #[must_use]
    pub const fn outstanding(&self) -> Option<RequestKey> {
        match self {
            Self::Loading { key, .. } => Some(*key),
            Self::Empty | Self::Ready(_) | Self::Failed(_) => None,
        }
    }

    /// Content that is safe to use for a new action; refreshing or failed
    /// status never authorizes a write from stale facts.
    #[must_use]
    pub const fn ready(&self) -> Option<&T> {
        match self {
            Self::Ready(value) => Some(value),
            Self::Empty | Self::Loading { .. } | Self::Failed(_) => None,
        }
    }

    /// Content to draw while a new read is in flight.
    #[must_use]
    pub const fn shown(&self) -> Option<&T> {
        match self {
            Self::Ready(value)
            | Self::Loading {
                previous: Some(value),
                ..
            } => Some(value),
            Self::Empty | Self::Loading { previous: None, .. } | Self::Failed(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_reads_supersede_old_and_duplicate_answers() {
        let mut keys = RequestKeys::default();
        let (old, new) = (keys.issue().unwrap(), keys.issue().unwrap());
        let mut read = Reading::default();
        read.start(old);
        read.start(new);
        assert!(!read.accept(old, Ok("stale branch")));
        assert!(read.accept(new, Ok("current branch")));
        assert!(!read.accept(new, Ok("duplicate")));
        assert_eq!(read.ready(), Some(&"current branch"));
    }

    #[test]
    fn refresh_keeps_the_row_but_never_authorizes_a_write_from_stale_facts() {
        let mut keys = RequestKeys::default();
        let mut read = Reading::Ready("old");
        let key = keys.issue().unwrap();
        read.start(key);
        assert_eq!(read.shown(), Some(&"old"));
        assert_eq!(read.ready(), None);
        assert!(read.accept(key, Err(Problem::Older)));
        assert_eq!(read.shown(), None);
        assert!(matches!(read, Reading::Failed(Problem::Older)));
    }

    #[test]
    fn recreated_repository_cannot_accept_a_previous_incarnations_reply() {
        let mut keys = RequestKeys::default();
        let old = keys.issue().unwrap();
        let mut original: Reading<&str> = Reading::default();
        original.start(old);
        let mut reopened = Reading::default();
        reopened.start(keys.issue().unwrap());
        assert!(!reopened.accept(old, Ok("old incarnation")));
        assert_eq!(reopened.ready(), None);
    }

    #[test]
    fn only_a_read_in_flight_has_an_outstanding_key() {
        let mut keys = RequestKeys::default();
        let key = keys.issue().unwrap();
        let mut read: Reading<&str> = Reading::default();
        assert_eq!(read.outstanding(), None);
        read.start(key);
        assert_eq!(read.outstanding(), Some(key));
        assert!(read.accept(key, Ok("answered")));
        assert_eq!(read.outstanding(), None);
    }

    #[test]
    fn keys_never_wrap_or_reuse_on_exhaustion() {
        let mut keys = RequestKeys(u64::MAX.saturating_sub(1));
        assert_eq!(keys.issue().unwrap(), RequestKey(u64::MAX));
        assert_eq!(keys.issue(), Err(KeysExhausted));
        assert_eq!(keys.issue(), Err(KeysExhausted));
    }
}
