//! Natural-content geometry and input around the existing sheet engine.
//! The host owns its one focus, opening admission, surface paint and dismissal.
mod element;
#[cfg(test)]
mod tests;
use super::snap::listener::{Admission, OnRelease, listener_when};
use super::snap::{self, Origin, Percentage, Release, Stop};
pub use element::Fitted;
use gpui::{AnyElement, App, IntoElement, ScrollHandle, Window};
use std::{cell::RefCell, rc::Rc};

/// Owner dismissal, called only after its existing admission accepts a pan.
pub type Closed = Rc<dyn Fn(&mut Window, &mut App)>;
/// Input uses the host's existing per-open fence; no second lease is created.
#[derive(Clone)]
pub struct Handlers {
    /// Refuse stale input before geometry, scroll or pointer capture changes.
    pub admit: Admission,
    /// The host decides whether pan-down can actually close its surface.
    pub closed: Closed,
}
impl std::fmt::Debug for Handlers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handlers").finish_non_exhaustive()
    }
}
/// Persistent geometry/scroll only. A host resets it at each explicit open.
#[derive(Debug, Default)]
pub struct State {
    motion: Rc<RefCell<snap::State>>,
    scroll: ScrollHandle,
}
impl State {
    /// Start without layout, focus, timers or an application identity.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Forget the previous geometry on the owner's explicit reopen or close.
    /// Retained frames keep only their old motion; the host also fences input.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    /// The same native handle the caller's bounded body must track.
    #[must_use]
    pub const fn scroll(&self) -> &ScrollHandle {
        &self.scroll
    }
    /// Cap/position one already-composed panel with its natural header/body.
    /// Menus pass the reference80-percent cap. The panel supplies its own
    /// shared frame paint; this transparent delegate installs no focus or keys.
    #[must_use]
    pub fn element(
        &self,
        child: impl IntoElement,
        cap: Percentage,
        window: &Window,
        handlers: &Handlers,
    ) -> Fitted {
        element::fitted(child, cap, window, &self.motion, &handlers.admit)
    }
    /// Overlay on the handle or bounded scroller, never on an editable field.
    /// The same listener owns slop, velocity, capture and wheel-scroll handoff.
    #[must_use]
    pub fn input(&self, origin: Origin, handlers: &Handlers) -> AnyElement {
        let motion = Rc::clone(&self.motion);
        let admit = Rc::clone(&handlers.admit);
        let closed = Rc::clone(&handlers.closed);
        let released: OnRelease = Rc::new(move |release, window, cx| {
            if *release == Release::Pan(Stop::Closed) && admit(window, cx) {
                // Unit owner callbacks can refuse; actual-close ACK removes the
                // frame. Retain this control until that happens, like Dialog.
                motion.borrow_mut().retain_compact();
                closed(window, cx);
            }
        });
        listener_when(
            &self.motion,
            origin,
            &self.scroll,
            &released,
            &handlers.admit,
        )
    }
}
