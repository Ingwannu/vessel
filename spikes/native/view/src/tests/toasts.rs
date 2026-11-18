//! `web/src/ui/toast-life.test.ts`: one toast at a time; a new one
//! replaces the one up with its own life (2.2 s, an error 3.2 s), and a
//! replaced toast's dismissal leaves the newer one.

use std::time::{Duration, Instant};

use crate::toasts::{ERROR_LIFE, TOAST_LIFE, ToastTone, Toasts};

#[test]
fn a_new_toast_replaces_the_one_up_and_runs_its_own_life() {
    assert_eq!(TOAST_LIFE.as_millis(), 2200);
    assert_eq!(ERROR_LIFE.as_millis(), 3200);
    let mut toasts = Toasts::default();
    let now = Instant::now();
    let first = toasts.push("first", ToastTone::Error, now);
    let later = now.checked_add(Duration::from_millis(3000)).unwrap();
    let second = toasts.push("second", ToastTone::Ok, later);
    assert_eq!(second.after(), TOAST_LIFE);
    // The first's 3.2 s pass: the second stays.
    let first_end = now.checked_add(ERROR_LIFE).unwrap();
    assert_eq!(toasts.wake(first, first_end), None);
    assert_eq!(
        toasts.shown().map(|toast| toast.text.as_str()),
        Some("second")
    );
    // Dismissing the replaced toast leaves the newer one.
    toasts.dismiss(first.id());
    assert_eq!(toasts.shown().map(|toast| toast.id), Some(second.id()));
    let second_end = later.checked_add(TOAST_LIFE).unwrap();
    assert_eq!(toasts.wake(second, second_end), None);
    assert!(toasts.shown().is_none());
    // A new one never takes a gone one's number.
    let again = toasts.push("Killed 9c1d", ToastTone::Ok, later);
    assert!(again.id() > second.id());
}
