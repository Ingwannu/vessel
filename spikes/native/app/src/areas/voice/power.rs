//! Generation-owned GPUI sleep inhibition, staged with the voice controller.
//!
//! Pending work and acquired guards share the same owner. No completion starts
//! capture; sleep callbacks perform only the caller's immediate local cleanup.

use gpui::{ActivityGuard, App, Subscription, Task};
use std::{cell::RefCell, rc::Rc};
use view::voice::Generation;

const REASON: &str = "Vessel spoken conversation";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Status {
    Pending,
    Held,
    Unavailable,
    Released,
}

struct Inner {
    status: Status,
    pending: Option<Task<()>>,
    guard: Option<ActivityGuard>,
}
impl Inner {
    fn release(&mut self) {
        // Mark closed before dropping work: a late completion cannot reclaim it.
        self.status = Status::Released;
        self.pending.take();
        self.guard.take();
    }
}

pub(super) struct Power {
    generation: Generation,
    inner: Rc<RefCell<Inner>>,
    sleep: Option<Subscription>,
}
impl Power {
    pub(super) fn request(
        generation: Generation,
        mut local_stop: impl FnMut() + 'static,
        cx: &App,
    ) -> Self {
        let inner = Rc::new(RefCell::new(Inner {
            status: Status::Pending,
            pending: None,
            guard: None,
        }));
        let acquisition = cx.prevent_idle_sleep(REASON);
        let pending = cx.foreground_executor().spawn({
            let weak = Rc::downgrade(&inner);
            async move {
                let result = acquisition.await;
                let Some(inner) = weak.upgrade() else { return };
                let mut inner = inner.borrow_mut();
                if inner.status != Status::Pending {
                    // Dropping a late guard releases it; it has no capture path.
                    return;
                }
                match result {
                    Ok(guard) => {
                        inner.guard = Some(guard);
                        inner.status = Status::Held;
                    }
                    Err(_private) => inner.status = Status::Unavailable,
                }
            }
        });
        inner.borrow_mut().pending = Some(pending);
        let sleep = cx.on_system_sleep({
            let weak = Rc::downgrade(&inner);
            move |_| {
                let Some(inner) = weak.upgrade() else { return };
                {
                    let mut inner = inner.borrow_mut();
                    if inner.status == Status::Released {
                        return;
                    }
                    inner.release();
                }
                // Do not retain a RefCell borrow across controller cleanup.
                // This closure must fence devices/uploads, never await a node.
                local_stop();
            }
        });
        Self {
            generation,
            inner,
            sleep: Some(sleep),
        }
    }
    pub(super) const fn generation(&self) -> Generation {
        self.generation
    }
    pub(super) fn status(&self) -> Status {
        self.inner.borrow().status
    }
    pub(super) fn release(&mut self) {
        self.sleep.take();
        self.inner.borrow_mut().release();
    }
}
impl Drop for Power {
    fn drop(&mut self) {
        self.release();
    }
}

#[cfg(test)]
mod tests;
