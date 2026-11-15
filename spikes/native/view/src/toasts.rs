//! One current toast (Paseo toast-host.tsx:66, 80–118, 127). All variants
//! share the same keyed monotonic clock, so hover and replacement cannot let
//! an old callback dismiss a newer card. No system clock is read in this model.
//!
//! [Decision Log]
//! - Purpose: match the reference's single toast and remaining-time hover pause.
//! - Constraint: existing Ok callers mean success, while Error calls are the
//!   error helper; advisory callers can retain an explicit reading duration.
//! - Alternatives: stacked independent timers or restarting on every hover-out.
//! - Choice: one bounded slot, exact id/phase checks, explicit sticky/custom life.
//! - Why: replacement is global and a stale timer never owns its successor.
//! - Impact: default 2200ms, error helper 3200ms, empty messages ignored.
use std::time::{Duration, Instant};
/// Paseo's default lifetime for show/copy feedback.
pub const TOAST_LIFE: Duration = Duration::from_millis(2200);
/// Explicit reading time used by unavailable-action guidance.
pub const INFO_LIFE: Duration = Duration::from_millis(3200);
/// Paseo's error helper lifetime.
pub const ERROR_LIFE: Duration = Duration::from_millis(3200);
/// What a toast says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastTone {
    /// A neutral message with no default icon.
    Default,
    /// Something went as asked (Paseo's success variant).
    Ok,
    /// Advisory explanation; the action was not attempted.
    Info,
    /// Something needs attention.
    Warning,
    /// Something failed; the error helper supplies its own reading time.
    Error,
}
/// Explicit duration selection, including a persistent message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifetime {
    /// The ordinary 2200ms show lifetime.
    Default,
    /// Remain until dismissed or replaced.
    Sticky,
    /// A caller's intentional reading time, such as provider warnings.
    Timed(Duration),
}
/// One toast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    /// Its non-reused dismissal identity.
    pub id: u64,
    /// Its semantic appearance.
    pub tone: ToastTone,
    /// Whole localized message.
    pub text: String,
}
/// One bounded toast slot and its matching optional countdown.
#[derive(Debug, Clone, Default)]
pub struct Toasts {
    list: Vec<Toast>,
    last: u64,
    lifetime: Option<Duration>,
    clock: Option<ToastClock>,
}
impl Toasts {
    /// Show feedback; Error is the 3200ms error helper, other variants use
    /// 2200ms. Empty messages leave the current toast untouched.
    pub fn push(&mut self, text: impl Into<String>, tone: ToastTone) -> (u64, Option<Duration>) {
        self.push_with(
            text,
            tone,
            if tone == ToastTone::Error {
                Lifetime::Timed(ERROR_LIFE)
            } else {
                Lifetime::Default
            },
        )
    }
    /// Show one message with explicit timing. The returned id is a no-op for
    /// ignored whitespace or an exhausted identity counter.
    pub fn push_with(
        &mut self,
        text: impl Into<String>,
        tone: ToastTone,
        lifetime: Lifetime,
    ) -> (u64, Option<Duration>) {
        let text = text.into();
        if text.trim().is_empty() {
            return (self.last, None);
        }
        let Some(id) = self.last.checked_add(1) else {
            return (self.last, None);
        };
        self.last = id;
        self.list.clear();
        self.list.push(Toast { id, tone, text });
        self.clock = None;
        self.lifetime = match lifetime {
            Lifetime::Default => Some(TOAST_LIFE),
            Lifetime::Sticky => None,
            Lifetime::Timed(duration) => Some(duration),
        };
        (id, self.lifetime)
    }
    /// Dismiss only the matching current toast.
    pub fn dismiss(&mut self, id: u64) {
        if self.list.first().is_some_and(|toast| toast.id == id) {
            self.list.clear();
            self.clock = None;
            self.lifetime = None;
        }
    }
    /// At most one card; retained as a slice for existing renderer callers.
    #[must_use]
    pub fn list(&self) -> &[Toast] {
        &self.list
    }
    /// Start the current non-sticky toast's injected monotonic clock.
    pub fn start(&mut self, id: u64, now: Instant) -> Option<ToastWake> {
        self.list.first().filter(|toast| toast.id == id)?;
        let remaining = self.lifetime?;
        if self.clock.as_ref().is_some_and(|clock| clock.id == id) {
            return None;
        }
        self.clock = Some(ToastClock {
            id,
            phase: ToastPhase::Running {
                since: now,
                remaining,
            },
        });
        Some(ToastWake {
            id,
            since: now,
            after: remaining,
        })
    }
    /// Pause/resume remaining time for any variant; stale ids do nothing.
    pub fn hover(&mut self, id: u64, hover: ToastHover, now: Instant) -> Option<ToastWake> {
        let clock = self.clock.as_mut().filter(|clock| clock.id == id)?;
        match (clock.phase, hover) {
            (ToastPhase::Running { since, remaining }, ToastHover::Enter) => {
                clock.phase = ToastPhase::Paused(
                    remaining.saturating_sub(now.saturating_duration_since(since)),
                );
                None
            }
            (ToastPhase::Paused(remaining), ToastHover::Leave) => {
                clock.phase = ToastPhase::Running {
                    since: now,
                    remaining,
                };
                Some(ToastWake {
                    id,
                    since: now,
                    after: remaining,
                })
            }
            (ToastPhase::Running { .. }, ToastHover::Leave)
            | (ToastPhase::Paused(_), ToastHover::Enter) => None,
        }
    }
    /// Only the current running phase can dismiss; early wakes keep remainder.
    pub fn wake(&mut self, wake: ToastWake, now: Instant) -> Option<ToastWake> {
        let clock = self.clock.as_ref().filter(|clock| clock.id == wake.id)?;
        let ToastPhase::Running { since, remaining } = clock.phase else {
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
/// Pointer presence, without toolkit types in lifetime policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastHover {
    /// Pause while the person reads.
    Enter,
    /// Resume the time remaining.
    Leave,
}
/// A scheduled phase, protected against stale hover/replacement callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToastWake {
    id: u64,
    since: Instant,
    after: Duration,
}
impl ToastWake {
    /// Executor delay; the app supplies the actual clock at wake.
    #[must_use]
    pub const fn after(self) -> Duration {
        self.after
    }
}
#[derive(Debug, Clone)]
struct ToastClock {
    id: u64,
    phase: ToastPhase,
}
#[derive(Debug, Clone, Copy)]
enum ToastPhase {
    Running { since: Instant, remaining: Duration },
    Paused(Duration),
}
#[cfg(test)]
mod info_tests {
    use super::*;
    #[test]
    fn default_and_error_helper_lifetimes_follow_paseo() {
        let mut toasts = Toasts::default();
        for (tone, expected) in [
            (ToastTone::Ok, Some(TOAST_LIFE)),
            (ToastTone::Info, Some(TOAST_LIFE)),
            (ToastTone::Error, Some(ERROR_LIFE)),
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
        assert_eq!(life, Some(TOAST_LIFE));
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
        let wake = toasts.start(id, now).unwrap();
        assert_eq!(wake.after(), Duration::from_millis(2200));
        let early = toasts
            .wake(wake, now + Duration::from_millis(2199))
            .unwrap();
        assert_eq!(early.after(), Duration::from_millis(1));
        assert_eq!(toasts.list().len(), 1);
        assert_eq!(toasts.wake(early, now + Duration::from_millis(2200)), None);
        assert_eq!(toasts.list(), &[]);
    }
    #[test]
    fn hover_keeps_guidance_visible_and_resume_uses_remaining_time_without_old_timer_interference()
    {
        let mut toasts = Toasts::default();
        let (id, _) = toasts.push("Advice", ToastTone::Info);
        let now = Instant::now();
        let first = toasts.start(id, now).unwrap();
        assert_eq!(
            toasts.hover(id, ToastHover::Enter, now + Duration::from_millis(1200)),
            None
        );
        assert_eq!(toasts.wake(first, now + Duration::from_secs(5)), None);
        assert_eq!(toasts.list().len(), 1);
        let resumed = toasts
            .hover(id, ToastHover::Leave, now + Duration::from_secs(5))
            .unwrap();
        assert_eq!(resumed.after(), Duration::from_millis(1000));
        assert_eq!(toasts.wake(first, now + Duration::from_millis(5100)), None);
        assert_eq!(toasts.list().len(), 1);
        assert_eq!(toasts.wake(resumed, now + Duration::from_secs(6)), None);
        assert_eq!(toasts.list(), &[]);
    }
    #[test]
    fn replacement_removes_every_older_variant_and_rejects_stale_callbacks() {
        let mut toasts = Toasts::default();
        let now = Instant::now();
        let (_legacy, _) = toasts.push("Error", ToastTone::Error);
        let (first, _) = toasts.push("Advice", ToastTone::Info);
        let old = toasts.start(first, now).unwrap();
        let (second, _) = toasts.push("Advice", ToastTone::Info);
        let current = toasts
            .start(second, now + Duration::from_millis(100))
            .unwrap();
        assert_eq!(
            toasts
                .list()
                .iter()
                .map(|toast| toast.id)
                .collect::<Vec<_>>(),
            [second]
        );
        assert_eq!(toasts.hover(first, ToastHover::Enter, now), None);
        assert_eq!(toasts.wake(old, now + Duration::from_millis(2200)), None);
        assert_eq!(toasts.list().len(), 1);
        assert_eq!(
            toasts.wake(current, now + Duration::from_millis(2300)),
            None
        );
        assert_eq!(
            toasts
                .list()
                .iter()
                .map(|toast| toast.id)
                .collect::<Vec<_>>(),
            []
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
        let (_error, _) = toasts.push("Error", ToastTone::Error);
        let (old, _) = toasts.push("Old advice", ToastTone::Info);
        let old_wake = toasts.start(old, now).unwrap();
        assert_eq!(
            toasts.hover(old, ToastHover::Enter, now + Duration::from_millis(1000)),
            None
        );
        let (new, _) = toasts.push("New advice", ToastTone::Info);
        let new_wake = toasts
            .start(new, now + Duration::from_millis(1000))
            .unwrap();
        assert_eq!(
            toasts.hover(old, ToastHover::Leave, now + Duration::from_millis(3000)),
            None
        );
        assert_eq!(
            toasts.wake(old_wake, now + Duration::from_millis(2200)),
            None
        );
        assert_eq!(toasts.start(new, now + Duration::from_millis(4000)), None);
        assert_eq!(
            toasts.wake(new_wake, now + Duration::from_millis(5200)),
            None
        );
        assert_eq!(
            toasts
                .list()
                .iter()
                .map(|toast| toast.id)
                .collect::<Vec<_>>(),
            []
        );
    }
    #[test]
    fn dismissing_paused_advice_cannot_reappear_on_a_late_hover_out() {
        let mut toasts = Toasts::default();
        let now = Instant::now();
        let (id, _) = toasts.push("Advice", ToastTone::Info);
        let wake = toasts.start(id, now).unwrap();
        assert_eq!(
            toasts.hover(id, ToastHover::Enter, now + Duration::from_millis(500)),
            None
        );
        toasts.dismiss(id);
        assert_eq!(
            toasts.hover(id, ToastHover::Leave, now + Duration::from_millis(1000)),
            None
        );
        assert_eq!(toasts.wake(wake, now + Duration::from_secs(4)), None);
        assert_eq!(toasts.list(), &[]);
    }
}
