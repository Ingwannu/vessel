//! Toasts (the page's `ui/toasts.tsx`, DESIGN.md §6.10): a stack of short
//! lines, oldest first. One that says something went as asked leaves by
//! itself after [`TOAST_LIFE`]; an error stays until it is dismissed.

// [Decision Log]
// - Purpose: source Info guidance pauses its remaining 3200ms on hover.
// - Constraint: no clock I/O in view; legacy Ok4s/Error-persistent stay.
// - Alternatives: restart on hover-out, or let an old timeout dismiss a new card.
// - Choice: injected monotonic time, keyed running phases, one replaceable Info.
// - Why: timeout/id/hover-out from a replaced card cannot affect its successor.
// - Cost: Info replaces only older Info to preserve legacy stacked callers;
//   Paseo replaces every kind. This is a compatibility departure, not parity.

use std::time::{Duration, Instant};

/// How long a legacy success toast stays; Info has its own reading lifetime.
pub const TOAST_LIFE: Duration = Duration::from_secs(4);

/// Paseo's unavailable action guidance (`git/use-actions.tsx`), distinct from
/// the legacy four-second success lifetime.
pub const INFO_LIFE: Duration = Duration::from_millis(3200);

/// What a toast says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastTone {
    /// Something went as asked.
    Ok,
    /// Advisory explanation; the action was not attempted.
    Info,
    /// Something failed: it stays until dismissed.
    Error,
}

/// One toast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    /// Its own number, to dismiss it by.
    pub id: u64,
    /// What it says.
    pub tone: ToastTone,
    /// Its words.
    pub text: String,
}

/// The toasts on screen.
#[derive(Debug, Clone, Default)]
pub struct Toasts {
    list: Vec<Toast>,
    last: u64,
    info: Option<InfoClock>,
}

impl Toasts {
    /// Show `text`; its id, and when it leaves by itself (an error does
    /// not).
    pub fn push(&mut self, text: impl Into<String>, tone: ToastTone) -> (u64, Option<Duration>) {
        if tone == ToastTone::Info {
            // Only this additive profile replaces its predecessor. Legacy
            // success/error callers keep their existing stack and lifetimes.
            self.list.retain(|toast| toast.tone != ToastTone::Info);
            self.info = None;
        }
        self.last = self.last.saturating_add(1);
        let id = self.last;
        self.list.push(Toast {
            id,
            tone,
            text: text.into(),
        });
        let life = match tone {
            ToastTone::Ok => Some(TOAST_LIFE),
            ToastTone::Info => Some(INFO_LIFE),
            ToastTone::Error => None,
        };
        (id, life)
    }

    /// Take toast `id` away (its time is up, or it was dismissed); one
    /// already gone is no matter.
    pub fn dismiss(&mut self, id: u64) {
        self.list.retain(|toast| toast.id != id);
        if self.info.as_ref().is_some_and(|clock| clock.id == id) {
            self.info = None;
        }
    }

    /// The toasts, oldest first.
    #[must_use]
    pub fn list(&self) -> &[Toast] {
        &self.list
    }
}

/// Pointer presence, without any toolkit type in the lifetime policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastHover {
    /// Stop the countdown while the person reads the guidance.
    Enter,
    /// Resume the time that remained, rather than start another full lifetime.
    Leave,
}

/// One scheduled wake; its start instant also identifies the running phase.
/// A timeout from before a hover or replacement cannot dismiss a later phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InfoWake {
    id: u64,
    since: Instant,
    after: Duration,
}
impl InfoWake {
    /// The executor's delay; the app supplies the monotonic clock on wake.
    #[must_use]
    pub const fn after(self) -> Duration {
        self.after
    }
}

#[derive(Debug, Clone)]
struct InfoClock {
    id: u64,
    phase: InfoPhase,
}
#[derive(Debug, Clone, Copy)]
enum InfoPhase {
    Running { since: Instant, remaining: Duration },
    Paused(Duration),
}

impl Toasts {
    /// Start only the current Info toast. `now` comes from the app; view reads
    /// no system clock and the legacy four-second success path stays unchanged.
    pub fn start_info(&mut self, id: u64, now: Instant) -> Option<InfoWake> {
        let _current = self
            .list
            .iter()
            .find(|toast| toast.id == id && toast.tone == ToastTone::Info)?;
        if self.info.as_ref().is_some_and(|clock| clock.id == id) {
            return None;
        }
        self.info = Some(InfoClock {
            id,
            phase: InfoPhase::Running {
                since: now,
                remaining: INFO_LIFE,
            },
        });
        Some(InfoWake {
            id,
            since: now,
            after: INFO_LIFE,
        })
    }

    /// Pause/resume the current guidance, never an older card at the same place.
    pub fn hover_info(&mut self, id: u64, hover: ToastHover, now: Instant) -> Option<InfoWake> {
        let clock = self.info.as_mut().filter(|clock| clock.id == id)?;
        match (&clock.phase, hover) {
            (InfoPhase::Running { since, remaining }, ToastHover::Enter) => {
                clock.phase = InfoPhase::Paused(
                    remaining.saturating_sub(now.saturating_duration_since(*since)),
                );
                None
            }
            (InfoPhase::Paused(remaining), ToastHover::Leave) => {
                let remaining = *remaining;
                clock.phase = InfoPhase::Running {
                    since: now,
                    remaining,
                };
                Some(InfoWake {
                    id,
                    since: now,
                    after: remaining,
                })
            }
            (InfoPhase::Running { .. }, ToastHover::Leave)
            | (InfoPhase::Paused(_), ToastHover::Enter) => None,
        }
    }

    /// Recheck the matching running phase at its wake. Paused, replaced and
    /// superseded timers do nothing; an early executor wake waits the remainder.
    pub fn wake_info(&mut self, wake: InfoWake, now: Instant) -> Option<InfoWake> {
        let clock = self.info.as_ref().filter(|clock| clock.id == wake.id)?;
        let InfoPhase::Running { since, remaining } = clock.phase else {
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
            Some(InfoWake {
                after: remaining,
                ..wake
            })
        }
    }
}

#[cfg(test)]
mod info_tests {
    use super::*;
    #[test]
    fn advice_has_its_own_exact_lifetime_and_legacy_success_error_lifetimes_stay() {
        let mut toasts = Toasts::default();
        for (tone, expected) in [
            (ToastTone::Ok, Some(Duration::from_secs(4))),
            (ToastTone::Info, Some(Duration::from_millis(3200))),
            (ToastTone::Error, None),
        ] {
            let (_, life) = toasts.push("The complete localized guidance.", tone);
            assert_eq!(life, expected);
        }
    }
    #[test]
    fn an_old_advice_timeout_cannot_dismiss_a_new_advice_even_with_the_same_words() {
        let mut toasts = Toasts::default();
        let (old, _) = toasts.push("There are no local commits to push.", ToastTone::Info);
        let (new, life) = toasts.push("There are no local commits to push.", ToastTone::Info);
        assert_ne!(new, old);
        toasts.dismiss(old);
        assert_eq!(
            toasts.list(),
            &[Toast {
                id: new,
                tone: ToastTone::Info,
                text: "There are no local commits to push.".into(),
            }]
        );
        assert_eq!(life, Some(INFO_LIFE));
        toasts.dismiss(new);
        assert_eq!(toasts.list(), &[]);
    }
}

#[cfg(test)]
mod info_clock_tests {
    use super::*;
    #[test]
    fn exact_deadline_and_early_wakes_do_not_round_away_the_remaining_millisecond() {
        let mut toasts = Toasts::default();
        let (id, _) = toasts.push("Advice", ToastTone::Info);
        let now = Instant::now();
        let wake = toasts.start_info(id, now).unwrap();
        assert_eq!(wake.after(), Duration::from_millis(3200));
        let early = toasts
            .wake_info(wake, now + Duration::from_millis(3199))
            .unwrap();
        assert_eq!(early.after(), Duration::from_millis(1));
        assert_eq!(toasts.list().len(), 1);
        assert_eq!(
            toasts.wake_info(early, now + Duration::from_millis(3200)),
            None
        );
        assert_eq!(toasts.list(), &[]);
    }
    #[test]
    fn hover_keeps_guidance_visible_and_resume_uses_remaining_time_without_old_timer_interference()
    {
        let mut toasts = Toasts::default();
        let (id, _) = toasts.push("Advice", ToastTone::Info);
        let now = Instant::now();
        let first = toasts.start_info(id, now).unwrap();
        assert_eq!(
            toasts.hover_info(id, ToastHover::Enter, now + Duration::from_millis(1200)),
            None
        );
        assert_eq!(toasts.wake_info(first, now + Duration::from_secs(5)), None);
        assert_eq!(toasts.list().len(), 1);
        let resumed = toasts
            .hover_info(id, ToastHover::Leave, now + Duration::from_secs(5))
            .unwrap();
        assert_eq!(resumed.after(), Duration::from_millis(2000));
        assert_eq!(
            toasts.wake_info(first, now + Duration::from_millis(5100)),
            None
        );
        assert_eq!(toasts.list().len(), 1);
        assert_eq!(
            toasts.wake_info(resumed, now + Duration::from_secs(7)),
            None
        );
        assert_eq!(toasts.list(), &[]);
    }
    #[test]
    fn replacing_info_preserves_legacy_cards_and_rejects_the_replaced_timeout_and_hover() {
        let mut toasts = Toasts::default();
        let now = Instant::now();
        let (legacy, _) = toasts.push("Error", ToastTone::Error);
        let (first, _) = toasts.push("Advice", ToastTone::Info);
        let old = toasts.start_info(first, now).unwrap();
        let (second, _) = toasts.push("Advice", ToastTone::Info);
        let current = toasts
            .start_info(second, now + Duration::from_millis(100))
            .unwrap();
        assert_eq!(
            toasts
                .list()
                .iter()
                .map(|toast| toast.id)
                .collect::<Vec<_>>(),
            [legacy, second]
        );
        assert_eq!(toasts.hover_info(first, ToastHover::Enter, now), None);
        assert_eq!(
            toasts.wake_info(old, now + Duration::from_millis(3200)),
            None
        );
        assert_eq!(toasts.list().len(), 2);
        assert_eq!(
            toasts.wake_info(current, now + Duration::from_millis(3300)),
            None
        );
        assert_eq!(
            toasts
                .list()
                .iter()
                .map(|toast| toast.id)
                .collect::<Vec<_>>(),
            [legacy]
        );
    }
}

#[cfg(test)]
mod hover_replacement_tests {
    use super::*;
    #[test]
    fn replacement_while_hovered_rejects_old_hover_out_and_still_expires_at_the_new_deadline() {
        let mut toasts = Toasts::default();
        let now = Instant::now();
        let (error, _) = toasts.push("Persistent error", ToastTone::Error);
        let (old, _) = toasts.push("Old advice", ToastTone::Info);
        let old_wake = toasts.start_info(old, now).unwrap();
        assert_eq!(
            toasts.hover_info(old, ToastHover::Enter, now + Duration::from_millis(1000)),
            None
        );
        let (new, _) = toasts.push("New advice", ToastTone::Info);
        let new_wake = toasts
            .start_info(new, now + Duration::from_millis(2000))
            .unwrap();
        assert_eq!(
            toasts.hover_info(old, ToastHover::Leave, now + Duration::from_millis(3000)),
            None
        );
        assert_eq!(
            toasts.wake_info(old_wake, now + Duration::from_millis(3200)),
            None
        );
        assert_eq!(
            toasts.start_info(new, now + Duration::from_millis(4000)),
            None
        );
        assert_eq!(
            toasts.wake_info(new_wake, now + Duration::from_millis(5200)),
            None
        );
        assert_eq!(
            toasts
                .list()
                .iter()
                .map(|toast| toast.id)
                .collect::<Vec<_>>(),
            [error]
        );
    }
    #[test]
    fn dismissing_paused_advice_cannot_reappear_on_a_late_hover_out() {
        let mut toasts = Toasts::default();
        let now = Instant::now();
        let (id, _) = toasts.push("Advice", ToastTone::Info);
        let wake = toasts.start_info(id, now).unwrap();
        assert_eq!(
            toasts.hover_info(id, ToastHover::Enter, now + Duration::from_millis(500)),
            None
        );
        toasts.dismiss(id);
        assert_eq!(
            toasts.hover_info(id, ToastHover::Leave, now + Duration::from_millis(1000)),
            None
        );
        assert_eq!(toasts.wake_info(wake, now + Duration::from_secs(4)), None);
        assert_eq!(toasts.list(), &[]);
    }
}
