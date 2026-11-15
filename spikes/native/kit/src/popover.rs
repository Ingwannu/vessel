//! Arbitrary anchored content on desktop, using the existing dialog on compact layouts.
//!
//! [Decision Log]
//! - Purpose: settings forms need a content popover, not an action-row menu.
//! - Constraints: one sheet/snap renderer and G4's actual-close focus policy.
//! - Alternatives: area-owned overlays or encoding controls as menu rows.
//! - Choice: an anchored popup plus the existing configured dialog; both share
//!   one `DialogState` and an opaque open lease checked before dismissal admission.
//! - Why: stale callbacks cannot acknowledge a newer surface, and arbitrary
//!   fields keep their own editing behavior without a second sheet engine.
//! - Cost: domain owners retain their context and explicitly admit dismissal.

mod draw;
mod input;
#[cfg(test)]
mod tests;

use crate::DialogState;
use gpui::{App, Bounds, Context, Pixels, Window};
use std::cell::Cell;
use std::rc::Rc;

pub use crate::dialog::DialogWidth as Width;
pub use draw::{PopoverContent, popover};

/// Which horizontal edge follows the measured trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    /// The surface starts at the trigger's left edge.
    Start,
    /// The surface ends at the trigger's right edge.
    End,
}

/// One opening's opaque identity. Closing/replacing it invalidates retained input.
#[derive(Debug, Clone)]
pub struct Lease(Rc<Cell<bool>>);

#[derive(Debug)]
struct Opening {
    anchor: Bounds<Pixels>,
    alignment: Alignment,
    lease: Lease,
}

impl Drop for Opening {
    fn drop(&mut self) {
        self.lease.0.set(false);
    }
}

/// One focus owner persists across desktop/compact presentation changes.
#[derive(Debug)]
pub struct PopoverState {
    opening: Option<Opening>,
    dialog: DialogState,
}

impl PopoverState {
    /// A closed surface, without a retained domain context.
    pub fn new(cx: &mut App) -> Self {
        Self {
            opening: None,
            dialog: DialogState::new(cx),
        }
    }

    /// Open from actual trigger bounds and return its identity for domain receipts.
    /// Replacing an opening finishes the previous focus/snap cycle first.
    #[must_use]
    pub fn open(
        &mut self,
        anchor: Bounds<Pixels>,
        alignment: Alignment,
        window: &mut Window,
        cx: &mut App,
    ) -> Lease {
        if let Some(previous) = &self.opening {
            previous.lease.0.set(false);
            self.dialog.dismissed(window, cx);
        }
        let lease = Lease(Rc::new(Cell::new(true)));
        self.opening = Some(Opening {
            anchor,
            alignment,
            lease: lease.clone(),
        });
        self.dialog.focus(window, cx);
        lease
    }

    /// The caller can correlate its captured host context without integer counters.
    #[must_use]
    pub fn is_current(&self, lease: &Lease) -> bool {
        self.opening
            .as_ref()
            .is_some_and(|current| Rc::ptr_eq(&current.lease.0, &lease.0))
    }

    /// Whether either presentation is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.opening.is_some()
    }

    /// Acknowledge an admitted actual close, preserving any newer focus owner.
    /// A stale lease cannot clear snap state or restore another opening's focus.
    pub fn close(&mut self, lease: &Lease, window: &mut Window, cx: &mut App) -> bool {
        if !self.is_current(lease) {
            return false;
        }
        lease.0.set(false);
        self.opening = None;
        self.dialog.dismissed(window, cx);
        true
    }
}

/// Domain admission happens before the shared control mutates state or focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dismissal {
    /// This exact opening may close now.
    Close,
    /// Pending work or a changed domain context keeps the current surface intact.
    KeepOpen,
}

/// Arbitrary feature bodies are an open extension point; the feature owns its
/// captured context and the control owns the actual close/focus acknowledgment.
pub type Admit<V> = Rc<dyn Fn(&mut V, &Lease, &mut Window, &mut Context<'_, V>) -> Dismissal>;

/// State access and domain admission, shared by Escape, outside and sheet close.
pub struct PopoverHandlers<V> {
    /// The exact caller-owned control state.
    pub state: fn(&mut V) -> &mut PopoverState,
    /// Refuse stale/pending domain decisions here; do not restore focus yourself.
    pub admit: Admit<V>,
}

impl<V> Clone for PopoverHandlers<V> {
    fn clone(&self) -> Self {
        Self {
            state: self.state,
            admit: Rc::clone(&self.admit),
        }
    }
}

impl<V> std::fmt::Debug for PopoverHandlers<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PopoverHandlers").finish_non_exhaustive()
    }
}

fn dismiss<V: 'static>(
    view: &mut V,
    lease: &Lease,
    handlers: &PopoverHandlers<V>,
    window: &mut Window,
    cx: &mut Context<'_, V>,
) {
    if !(handlers.state)(view).is_current(lease) {
        return;
    }
    match (handlers.admit)(view, lease, window, cx) {
        Dismissal::KeepOpen => {}
        Dismissal::Close => {
            // Admission may replace the opening or focus an unrelated field.
            // close rechecks the lease; G4's policy preserves that newer focus.
            let _closed = (handlers.state)(view).close(lease, window, cx);
            cx.notify();
        }
    }
}
