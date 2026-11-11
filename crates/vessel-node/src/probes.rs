//! `harness.options` (protocol 9, ADR 0022): what each agent offers, asked
//! of the agent itself and kept for a while.
//!
//! [Decision Log]
//! - Purpose: a new-session form shows each agent's models, efforts and
//!   permissions modes before anything runs. Asking costs an agent start
//!   (a few seconds); forms open often, and several pages may ask at once.
//! - Constraint: the answer depends on the harness and on the profile (an
//!   account offers its own models), and goes stale when the harness is
//!   upgraded. A page must never hang on it: a probe is bounded by the
//!   handshake timeout (`vessel_agent::probe`).
//! - Alternatives: probing at node start (slow starts, and stale until the
//!   next one); a cache task owning a map with request messages (a second
//!   actor for what one lock per key already serializes); caching failures
//!   (a person who signs in would wait out the cache).
//! - Choice: one slot per [`ProbeKey`], each behind its own async lock held
//!   across the probe, so concurrent callers for one key wait for the one
//!   probe and take its outcome, success or failure. Later callers take a
//!   success while it is younger than [`FRESH`] and the harness reports the
//!   same version; a failure is never reused by a caller that asked after
//!   it, so the next one probes again. The map of slots is a `std` mutex
//!   held only to find or add a slot, never across an `.await`.
//! - Cost: the version check runs `<harness> --version` for each call
//!   (`vessel_harness::discover`, as `harness.list` does), a short process
//!   per installed harness rather than an agent start.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use tokio::time::Instant;
use vessel_proto::{HarnessKind, HarnessOptions, ProfileName, WireError};

/// How long a probe's answer is reused.
pub(crate) const FRESH: Duration = Duration::from_secs(600);

/// What a probe's answer depends on.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ProbeKey {
    pub(crate) harness: HarnessKind,
    /// `None` is the harness's default profile.
    pub(crate) profile: Option<ProfileName>,
}

/// The harness's `--version` line when it was probed; `None` when it did
/// not answer one (then any version change goes unseen until [`FRESH`]
/// runs out).
pub(crate) type HarnessVersion = Option<String>;

/// The probes' answers, and where they run.
#[derive(Debug)]
pub(crate) struct Probes {
    /// The working directory of every probe (`DataDir::probe_dir`).
    dir: PathBuf,
    slots: Mutex<HashMap<ProbeKey, Arc<tokio::sync::Mutex<Slot>>>>,
}

/// One key's last probe.
#[derive(Debug, Default)]
struct Slot {
    last: Option<Probed>,
}

#[derive(Debug)]
struct Probed {
    /// When it finished, on the monotonic clock.
    finished: Instant,
    version: HarnessVersion,
    outcome: Result<HarnessOptions, WireError>,
}

impl Probes {
    pub(crate) fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            slots: Mutex::default(),
        }
    }

    /// Where probes run.
    pub(crate) const fn dir(&self) -> &PathBuf {
        &self.dir
    }

    /// `key`'s options: a recent answer for the same `version`, the answer
    /// of a probe that finished while this caller waited, or `probe`'s.
    pub(crate) async fn options(
        &self,
        key: ProbeKey,
        version: HarnessVersion,
        probe: impl AsyncFnOnce() -> Result<HarnessOptions, WireError>,
    ) -> Result<HarnessOptions, WireError> {
        let asked = Instant::now();
        let slot = Arc::clone(
            self.slots
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .entry(key)
                .or_default(),
        );
        // A `tokio` lock held across the probe on purpose: it is what makes
        // concurrent callers of one key share one probe (see the module's
        // Decision Log). Each key has its own, so other keys never wait.
        let mut slot = slot.lock().await;
        if let Some(last) = &slot.last
            && last.answers(asked, version.as_ref())
        {
            return last.outcome.clone();
        }
        let outcome = probe().await;
        slot.last = Some(Probed {
            finished: Instant::now(),
            version,
            outcome: outcome.clone(),
        });
        outcome
    }
}

impl Probed {
    /// Whether this answers a caller that asked at `asked` about a harness
    /// now at `version`.
    fn answers(&self, asked: Instant, version: Option<&String>) -> bool {
        // Finished after the caller asked, so while it waited for the slot:
        // the probe it waited for.
        if self.finished > asked {
            return true;
        }
        self.outcome.is_ok() && self.version.as_ref() == version && self.finished.elapsed() < FRESH
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use chrono::DateTime;
    use vessel_proto::ErrorCode;

    use super::*;

    fn key() -> ProbeKey {
        ProbeKey {
            harness: HarnessKind::ClaudeCode,
            profile: None,
        }
    }

    fn answer() -> HarnessOptions {
        HarnessOptions {
            options: Vec::new(),
            probed_at: DateTime::UNIX_EPOCH,
            abilities: None,
        }
    }

    #[tokio::test(start_paused = true)]
    async fn concurrent_callers_share_one_probe_and_a_fresh_answer_is_kept() {
        let probes = Probes::new(PathBuf::new());
        let started = AtomicUsize::new(0);
        let probe = async || {
            let _first = started.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_secs(3)).await;
            Ok(answer())
        };
        let version = || Some("1.0".to_owned());
        let (a, b) = tokio::join!(
            probes.options(key(), version(), probe),
            probes.options(key(), version(), probe)
        );
        assert_eq!((a, b), (Ok(answer()), Ok(answer())));
        assert_eq!(started.load(Ordering::SeqCst), 1, "one probe for both");

        let _kept = probes.options(key(), version(), probe).await;
        assert_eq!(started.load(Ordering::SeqCst), 1, "fresh: kept");
        let _upgraded = probes.options(key(), Some("1.1".to_owned()), probe).await;
        assert_eq!(started.load(Ordering::SeqCst), 2, "a new version probes");
        tokio::time::advance(FRESH).await;
        let _stale = probes.options(key(), Some("1.1".to_owned()), probe).await;
        assert_eq!(started.load(Ordering::SeqCst), 3, "stale: probed again");
    }

    #[tokio::test(start_paused = true)]
    async fn a_failure_is_shared_with_its_waiters_and_not_kept() {
        let probes = Probes::new(PathBuf::new());
        let started = AtomicUsize::new(0);
        let failing = async || {
            let _first = started.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_secs(3)).await;
            Err(WireError::new(ErrorCode::SpawnFailed, "not signed in"))
        };
        let (a, b) = tokio::join!(
            probes.options(key(), None, failing),
            probes.options(key(), None, failing)
        );
        assert!(a.is_err() && b.is_err());
        assert_eq!(started.load(Ordering::SeqCst), 1, "waiters share it");
        let _again = probes.options(key(), None, failing).await;
        assert_eq!(started.load(Ordering::SeqCst), 2, "a later caller retries");
    }
}
