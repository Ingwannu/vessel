//! A popup opening owns its body and subscription; kit owns placement/focus.
//! Retire inputs before dropping the body, then close the exact kit lease.
use super::{Opener, SurfaceEvent, SurfaceView, draw};
use gpui::AppContext as _;
use std::{cell::Cell, rc::Rc};
use vessel_app_kit::{
    Tokens,
    popover::{self, Alignment, Dismissal, Lease, PopoverContent, PopoverHandlers, Width},
    ui::{
        AnyElement, Bounds, Context, Entity, IntoElement as _, Pixels, Point, Subscription, Window,
    },
};
use vessel_app_view::plugins::{Problem, SurfaceProblem, SurfaceState};
use vessel_proto::plugins::{Caption, ContributionId};

/// Native form width is fixed presentation policy, never plugin CSS/code.
const FORM_WIDTH: u16 = 360;

#[derive(Clone, Copy)]
pub(super) struct Trigger {
    pub(super) at: Point<Pixels>,
    pub(super) bounds: Option<Bounds<Pixels>>,
}

/// A private tree of UI lifetimes, distinct from node/graph authorization.
/// Ancestor retirement also revokes a retained nested body without a counter.
pub(super) struct InputScope {
    live: Cell<bool>,
    parent: Option<Rc<Self>>,
}
impl InputScope {
    pub(super) fn root() -> Rc<Self> {
        Rc::new(Self {
            live: Cell::new(true),
            parent: None,
        })
    }
    fn child(parent: &Rc<Self>) -> Rc<Self> {
        Rc::new(Self {
            live: Cell::new(true),
            parent: Some(Rc::clone(parent)),
        })
    }
    pub(super) fn is_root(&self) -> bool {
        self.parent.is_none()
    }
    pub(super) fn is_live(&self) -> bool {
        self.live.get() && self.parent.as_ref().is_none_or(|parent| parent.is_live())
    }
    pub(super) fn retire(&self) {
        self.live.set(false);
    }
}

pub(super) struct Opening {
    pub(super) lease: Lease,
    pub(super) body: Entity<SurfaceView>,
    title: Caption,
    inputs: Rc<InputScope>,
    _subscription: Subscription,
}
impl Drop for Opening {
    fn drop(&mut self) {
        self.inputs.retire();
    }
}

impl SurfaceView {
    pub(super) fn queue_popover(
        &mut self,
        control: ContributionId,
        path: Vec<ContributionId>,
        anchor: Option<Bounds<Pixels>>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(anchor) = anchor else {
            cx.emit(SurfaceEvent::Rejected(SurfaceProblem::Value));
            return;
        };
        let intent = Rc::new(());
        self.pending_popup = Some(Rc::clone(&intent));
        if let Some(previous) = self.shown_popover.take() {
            let lease = previous.lease.clone();
            drop(previous);
            cx.defer_in(window, move |view, window, cx| {
                let _closed = view.popover.close(&lease, window, cx);
            });
        }
        cx.defer_in(window, move |view, window, cx| {
            if !view
                .pending_popup
                .as_ref()
                .is_some_and(|current| Rc::ptr_eq(current, &intent))
            {
                return;
            }
            view.pending_popup = None;
            if let Err(problem) = view.open_popover(&control, &path, anchor, window, cx) {
                cx.emit(SurfaceEvent::Rejected(problem));
            }
        });
    }

    fn open_popover(
        &mut self,
        control: &ContributionId,
        path: &[ContributionId],
        anchor: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Result<(), SurfaceProblem> {
        let (title, surface, state, registry) = self.with_registry(|registry| {
            let (title, surface) = self.state.popover(registry, control, path)?;
            Ok((
                title.clone(),
                surface.clone(),
                SurfaceState::new(registry, self.state.address().clone())?,
                self.registry.upgrade().ok_or(Problem::Stale)?,
            ))
        })?;
        // Invalidate the old body's queued input before any focus acknowledgement.
        if let Some(previous) = self.shown_popover.take() {
            let lease = previous.lease.clone();
            drop(previous);
            let _closed = self.popover.close(&lease, window, cx);
        }
        let lease = self.popover.open(anchor, Alignment::Start, window, cx);
        let inputs = InputScope::child(&self.inputs);
        let body_inputs = Rc::clone(&inputs);
        let opener: Opener = self.opener.clone();
        let body =
            cx.new(|cx| Self::build(&registry, state, surface, opener, body_inputs, window, cx));
        let captured = lease.clone();
        let subscription =
            cx.subscribe_in(&body, window, move |view, _body, event, _window, cx| {
                if !view.popover.is_current(&captured) || !view.inputs.is_live() {
                    return;
                }
                match event {
                    SurfaceEvent::Requested(request) => view.publish(Ok(request.clone()), cx),
                    SurfaceEvent::Rejected(problem) => cx.emit(SurfaceEvent::Rejected(*problem)),
                    SurfaceEvent::LinkRejected => cx.emit(SurfaceEvent::LinkRejected),
                }
            });
        self.shown_popover = Some(Opening {
            lease,
            body,
            title,
            inputs,
            _subscription: subscription,
        });
        cx.notify();
        Ok(())
    }

    pub(super) fn retire_popover(&mut self, lease: &Lease) -> Dismissal {
        if !self.inputs.is_live() || !self.popover.is_current(lease) {
            return Dismissal::KeepOpen;
        }
        // Taking the owner drops its subscription and revokes all descendants.
        self.shown_popover = None;
        Dismissal::Close
    }
}

pub(super) fn overlay(
    view: &SurfaceView,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, SurfaceView>,
) -> Option<AnyElement> {
    let shown = view.shown_popover.as_ref()?;
    let width = Width::new(FORM_WIDTH)?;
    popover::popover(
        &view.popover,
        PopoverContent::new(
            "plugin-declared-popover",
            shown.title.as_str().to_owned(),
            shown.body.clone().into_any_element(),
            width,
        ),
        &PopoverHandlers {
            state: |view: &mut SurfaceView| &mut view.popover,
            admit: Rc::new(|view, lease, _window, _cx| view.retire_popover(lease)),
        },
        tokens,
        window,
        draw::narrow(window),
        cx,
    )
}

#[cfg(test)]
mod tests {
    use super::InputScope;

    #[test]
    fn ancestor_retirement_reaches_a_separately_retained_nested_body() {
        let root = InputScope::root();
        let body = InputScope::child(&root);
        let nested = InputScope::child(&body);
        assert!(nested.is_live());
        body.retire();
        assert!(root.is_live());
        assert!(!nested.is_live());
        let successor = InputScope::child(&root);
        assert!(successor.is_live());
        root.retire();
        assert!(!successor.is_live());
    }
}
