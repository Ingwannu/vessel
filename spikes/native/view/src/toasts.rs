//! Toasts (the page's `ui/toast-life.ts`, DESIGN.md §6.10; Paseo's
//! `ToastHost`, `toast-host.tsx` 81-195): one at a time. A new toast
//! replaces the one up and starts its own life; a pointer resting on it
//! holds the life it has left.

// [Decision Log]
// - Purpose: TRACKER 802. Paseo shows one toast with its own duration
//   (2.2 s, an error 3.2 s); the app kept a stack in which a success ran
//   4 s and an error stayed until its close button, and only the git
//   guidance (Info) replaced its predecessor and paused on hover.
// - Constraint: no clock I/O in view; the app passes the monotonic `now`.
// - Alternatives: keep the list and cap it at one, leaving each old timer
//   to tell a gone toast from the shown one; or Paseo's one slot with the
//   keyed, phased clock the Info guidance already had.
// - Choice: one slot; every toast runs that clock (keyed by its id and the
//   instant its running phase began), so a wake, a hover or a dismissal
//   from a replaced toast cannot touch its successor.
// - Costs: a toast that arrives while another is read replaces it, an
//   error too, and an error leaves by itself after 3.2 s (Paseo's rule).
//   No caller needs Paseo's `durationMs: null`, so there is none here.

use std::time::{Duration, Instant};

/// Paseo's `DEFAULT_DURATION_MS`: a success, a copy.
pub const TOAST_LIFE: Duration = Duration::from_millis(2200);

/// Paseo's `toast.error` duration.
pub const ERROR_LIFE: Duration = Duration::from_millis(3200);

/// Paseo's unavailable action guidance (`git/use-actions.tsx`).
pub const INFO_LIFE: Duration = Duration::from_millis(3200);

/// What a toast says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastTone {
    /// Something went as asked (Paseo's `success`).
    Ok,
    /// Something was copied (Paseo's `toast.copied`).
    Copied,
    /// Advisory explanation; the action was not attempted.
    Info,
    /// Something failed.
    Error,
}

impl ToastTone {
    /// How long a toast of this tone stays by itself.
    #[must_use]
    pub const fn life(self) -> Duration {
        match self {
            Self::Ok | Self::Copied => TOAST_LIFE,
            Self::Info => INFO_LIFE,
            Self::Error => ERROR_LIFE,
        }
    }
}

/// One toast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    /// Its own number: its clock, its hover and its dismissal go by it.
    pub id: u64,
    /// What it says.
    pub tone: ToastTone,
    /// Its words.
    pub text: String,
}

/// The toast on screen, if any.
#[derive(Debug, Clone, Default)]
pub struct Toasts {
    shown: Option<Toast>,
    last: u64,
    clock: Option<Clock>,
}

/// Pointer presence, without any toolkit type in the lifetime policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastHover {
    /// Stop the countdown while the person reads.
    Enter,
    /// Resume the time that remained, rather than start another full life.
    Leave,
}

/// One scheduled wake; its start instant also identifies the running phase.
/// A timeout from before a hover or replacement cannot dismiss a later phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToastWake {
    id: u64,
    since: Instant,
    after: Duration,
}

impl ToastWake {
    /// The executor's delay; the app supplies the monotonic clock on wake.
    #[must_use]
    pub const fn after(self) -> Duration {
        self.after
    }

    /// The toast it is for.
    #[must_use]
    pub const fn id(self) -> u64 {
        self.id
    }
}

#[derive(Debug, Clone)]
struct Clock {
    id: u64,
    phase: Phase,
}

#[derive(Debug, Clone, Copy)]
enum Phase {
    Running { since: Instant, remaining: Duration },
    Paused(Duration),
}

impl Toasts {
    /// Show `text` in place of the toast up, its life starting `now`; the
    /// wake to schedule.
    pub fn push(&mut self, text: impl Into<String>, tone: ToastTone, now: Instant) -> ToastWake {
        self.last = self.last.saturating_add(1);
        let id = self.last;
        self.shown = Some(Toast {
            id,
            tone,
            text: text.into(),
        });
        let life = tone.life();
        self.clock = Some(Clock {
            id,
            phase: Phase::Running {
                since: now,
                remaining: life,
            },
        });
        ToastWake {
            id,
            since: now,
            after: life,
        }
    }

    /// Take toast `id` away; one already gone or replaced is no matter.
    pub fn dismiss(&mut self, id: u64) {
        if self.shown.as_ref().is_some_and(|toast| toast.id == id) {
            self.shown = None;
            self.clock = None;
        }
    }

    /// The toast up.
    #[must_use]
    pub const fn shown(&self) -> Option<&Toast> {
        self.shown.as_ref()
    }

    /// Pause or resume toast `id`'s life, never a replaced toast's.
    pub fn hover(&mut self, id: u64, hover: ToastHover, now: Instant) -> Option<ToastWake> {
        let clock = self.clock.as_mut().filter(|clock| clock.id == id)?;
        match (&clock.phase, hover) {
            (Phase::Running { since, remaining }, ToastHover::Enter) => {
                clock.phase = Phase::Paused(
                    remaining.saturating_sub(now.saturating_duration_since(*since)),
                );
                None
            }
            (Phase::Paused(remaining), ToastHover::Leave) => {
                let remaining = *remaining;
                clock.phase = Phase::Running {
                    since: now,
                    remaining,
                };
                Some(ToastWake {
                    id,
                    since: now,
                    after: remaining,
                })
            }
            (Phase::Running { .. }, ToastHover::Leave)
            | (Phase::Paused(_), ToastHover::Enter) => None,
        }
    }

    /// Recheck the matching running phase at its wake. Paused, replaced and
    /// superseded timers do nothing; an early executor wake waits the remainder.
    pub fn wake(&mut self, wake: ToastWake, now: Instant) -> Option<ToastWake> {
        let clock = self.clock.as_ref().filter(|clock| clock.id == wake.id)?;
        let Phase::Running { since, remaining } = clock.phase else {
            return None;
        };
        if since != wake.since {
            return None;
        }
        let remaining = remaining.saturating_sub(now.saturating_duration_since(since));
        if remaining.is_zero() {
            self.dismiss(wake.id);
            None
        } else {
            Some(ToastWake {
                after: remaining,
                ..wake
            })
        }
    }
}

#[cfg(test)]
fn at(now: Instant, millis: u64) -> Instant {
    now + Duration::from_millis(millis)
}

#[cfg(test)]
mod clock_tests {
    use super::*;

    #[test]
    fn each_tone_lives_as_paseo_says() {
        let mut toasts = Toasts::default();
        let now = Instant::now();
        for (tone, millis) in [
            (ToastTone::Ok, 2200),
            (ToastTone::Copied, 2200),
            (ToastTone::Info, 3200),
            (ToastTone::Error, 3200),
        ] {
            let wake = toasts.push("Words", tone, now);
            assert_eq!(wake.after(), Duration::from_millis(millis));
        }
    }

    #[test]
    fn exact_deadline_and_early_wakes_do_not_round_away_the_remaining_millisecond() {
        let mut toasts = Toasts::default();
        let now = Instant::now();
        let wake = toasts.push("Advice", ToastTone::Info, now);
        let early = toasts.wake(wake, at(now, 3199)).unwrap();
        assert_eq!(early.after(), Duration::from_millis(1));
        assert!(toasts.shown().is_some());
        assert_eq!(toasts.wake(early, at(now, 3200)), None);
        assert_eq!(toasts.shown(), None);
    }

    #[test]
    fn hover_keeps_the_toast_and_resume_uses_the_remaining_time() {
        let mut toasts = Toasts::default();
        let now = Instant::now();
        let first = toasts.push("Merged", ToastTone::Ok, now);
        let id = first.id();
        assert_eq!(toasts.hover(id, ToastHover::Enter, at(now, 1200)), None);
        assert_eq!(toasts.wake(first, at(now, 5000)), None);
        assert!(toasts.shown().is_some());
        let resumed = toasts.hover(id, ToastHover::Leave, at(now, 5000)).unwrap();
        assert_eq!(resumed.after(), Duration::from_millis(1000));
        assert_eq!(toasts.wake(first, at(now, 5100)), None);
        assert!(toasts.shown().is_some());
        assert_eq!(toasts.wake(resumed, at(now, 6000)), None);
        assert_eq!(toasts.shown(), None);
    }
}

#[cfg(test)]
mod replacement_tests {
    use super::*;

    #[test]
    fn a_new_toast_replaces_the_one_up_whatever_their_tones() {
        let mut toasts = Toasts::default();
        let now = Instant::now();
        let error = toasts.push("The machine is stopping", ToastTone::Error, now);
        let copied = toasts.push("Path copied", ToastTone::Copied, now);
        assert_ne!(copied.id(), error.id());
        assert_eq!(
            toasts.shown(),
            Some(&Toast {
                id: copied.id(),
                tone: ToastTone::Copied,
                text: "Path copied".into(),
            })
        );
        // The error's wake, hover and dismissal no longer reach anything.
        assert_eq!(toasts.wake(error, at(now, 3200)), None);
        assert_eq!(toasts.hover(error.id(), ToastHover::Enter, now), None);
        toasts.dismiss(error.id());
        assert_eq!(toasts.shown().map(|toast| toast.id), Some(copied.id()));
        assert_eq!(toasts.wake(copied, at(now, 2200)), None);
        assert_eq!(toasts.shown(), None);
    }

    #[test]
    fn replacement_while_hovered_rejects_old_hover_out_and_expires_at_the_new_deadline() {
        let mut toasts = Toasts::default();
        let now = Instant::now();
        let old = toasts.push("Old advice", ToastTone::Info, now);
        assert_eq!(
            toasts.hover(old.id(), ToastHover::Enter, at(now, 1000)),
            None
        );
        let new = toasts.push("New advice", ToastTone::Info, at(now, 2000));
        assert_eq!(
            toasts.hover(old.id(), ToastHover::Leave, at(now, 3000)),
            None
        );
        assert_eq!(toasts.wake(old, at(now, 3200)), None);
        assert!(toasts.shown().is_some());
        assert_eq!(toasts.wake(new, at(now, 5200)), None);
        assert_eq!(toasts.shown(), None);
    }

    #[test]
    fn dismissing_a_paused_toast_cannot_bring_it_back_on_a_late_hover_out() {
        let mut toasts = Toasts::default();
        let now = Instant::now();
        let wake = toasts.push("Advice", ToastTone::Info, now);
        assert_eq!(
            toasts.hover(wake.id(), ToastHover::Enter, at(now, 500)),
            None
        );
        toasts.dismiss(wake.id());
        assert_eq!(
            toasts.hover(wake.id(), ToastHover::Leave, at(now, 1000)),
            None
        );
        assert_eq!(toasts.wake(wake, at(now, 4000)), None);
        assert_eq!(toasts.shown(), None);
    }
}
