//! `web/src/ui/toasts.tsx`: a stack, oldest first; what went as asked
//! leaves after four seconds, an error stays until dismissed.

use crate::toasts::{TOAST_LIFE, ToastTone, Toasts};

#[test]
fn keeps_toasts_oldest_first_and_lets_only_what_went_well_leave_by_itself() {
    let mut toasts = Toasts::default();
    let (killed, life) = toasts.push("Killed 3f2a", ToastTone::Ok);
    assert_eq!(life, Some(TOAST_LIFE));
    assert_eq!(TOAST_LIFE.as_millis(), 4000);
    let (refused, life) = toasts.push("The machine is stopping", ToastTone::Error);
    assert_eq!(life, None);
    assert_ne!(killed, refused);
    let words: Vec<&str> = toasts
        .list()
        .iter()
        .map(|toast| toast.text.as_str())
        .collect();
    assert_eq!(words, ["Killed 3f2a", "The machine is stopping"]);
    toasts.dismiss(killed);
    toasts.dismiss(killed);
    assert_eq!(toasts.list().len(), 1);
    assert_eq!(toasts.list()[0].tone, ToastTone::Error);
    toasts.dismiss(refused);
    assert!(toasts.list().is_empty());
    // A new one never takes a gone one's number.
    let (again, _) = toasts.push("Killed 9c1d", ToastTone::Ok);
    assert!(again > refused);
}
