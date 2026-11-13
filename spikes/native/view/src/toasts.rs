//! Toasts (the page's `ui/toasts.tsx`, DESIGN.md §6.10): a stack of short
//! lines, oldest first. One that says something went as asked leaves by
//! itself after [`TOAST_LIFE`]; an error stays until it is dismissed.

use std::time::Duration;

/// How long a toast that is not an error stays.
pub const TOAST_LIFE: Duration = Duration::from_secs(4);

/// What a toast says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastTone {
    /// Something went as asked.
    Ok,
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
}

impl Toasts {
    /// Show `text`; its id, and when it leaves by itself (an error does
    /// not).
    pub fn push(&mut self, text: impl Into<String>, tone: ToastTone) -> (u64, Option<Duration>) {
        self.last = self.last.saturating_add(1);
        let id = self.last;
        self.list.push(Toast {
            id,
            tone,
            text: text.into(),
        });
        let life = match tone {
            ToastTone::Ok => Some(TOAST_LIFE),
            ToastTone::Error => None,
        };
        (id, life)
    }

    /// Take toast `id` away (its time is up, or it was dismissed); one
    /// already gone is no matter.
    pub fn dismiss(&mut self, id: u64) {
        self.list.retain(|toast| toast.id != id);
    }

    /// The toasts, oldest first.
    #[must_use]
    pub fn list(&self) -> &[Toast] {
        &self.list
    }
}
