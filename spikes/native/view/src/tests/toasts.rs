//! Paseo toast-host.tsx:66,80–118,127. Coordinator approved replacing the
//! old stack/4s/sticky-error assertions under A8 on 2026-11-15.
use crate::toasts::{ERROR_LIFE, Lifetime, TOAST_LIFE, ToastTone, Toasts};
#[test]
fn one_toast_replaces_every_variant_and_old_dismiss_is_harmless() {
    let mut toasts = Toasts::default();
    let (success, life) = toasts.push("Killed 3f2a", ToastTone::Ok);
    assert_eq!(life, Some(TOAST_LIFE));
    assert_eq!(TOAST_LIFE.as_millis(), 2200);
    let (error, life) = toasts.push("The machine is stopping", ToastTone::Error);
    assert_eq!(life, Some(ERROR_LIFE));
    assert_eq!(ERROR_LIFE.as_millis(), 3200);
    assert_ne!(success, error);
    assert_eq!(toasts.list().len(), 1);
    assert_eq!(toasts.list()[0].tone, ToastTone::Error);
    toasts.dismiss(success);
    assert_eq!(toasts.list()[0].id, error);
    toasts.dismiss(error);
    assert!(toasts.list().is_empty());
    let (again, _) = toasts.push("Killed 9c1d", ToastTone::Ok);
    assert!(again > error);
}
#[test]
fn whitespace_preserves_current_message_and_sticky_is_explicit() {
    let mut toasts = Toasts::default();
    let (sticky, life) = toasts.push_with("Keep this", ToastTone::Warning, Lifetime::Sticky);
    assert_eq!(life, None);
    assert!(toasts.start(sticky, std::time::Instant::now()).is_none());
    let _ignored = toasts.push(" \n\t", ToastTone::Error);
    assert_eq!(toasts.list()[0].id, sticky);
    toasts.dismiss(sticky);
    assert!(toasts.list().is_empty());
}
#[test]
fn every_timed_variant_pauses_and_resumes_its_remaining_time() {
    use crate::toasts::ToastHover;
    use std::time::{Duration, Instant};
    for tone in [
        ToastTone::Default,
        ToastTone::Info,
        ToastTone::Ok,
        ToastTone::Warning,
        ToastTone::Error,
    ] {
        let mut toasts = Toasts::default();
        let (id, _) = toasts.push_with("Read me", tone, Lifetime::Timed(Duration::from_secs(5)));
        let now = Instant::now();
        let original = toasts.start(id, now).unwrap();
        assert_eq!(
            toasts.hover(id, ToastHover::Enter, now + Duration::from_secs(1)),
            None
        );
        assert_eq!(toasts.wake(original, now + Duration::from_secs(20)), None);
        assert_eq!(toasts.list().len(), 1);
        let resumed = toasts
            .hover(id, ToastHover::Leave, now + Duration::from_secs(20))
            .unwrap();
        assert_eq!(resumed.after(), Duration::from_secs(4));
        assert_eq!(toasts.wake(original, now + Duration::from_secs(21)), None);
        assert_eq!(toasts.list().len(), 1);
        assert_eq!(toasts.wake(resumed, now + Duration::from_secs(24)), None);
        assert!(toasts.list().is_empty());
    }
}
