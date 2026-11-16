//! At most one owned hover timer in the existing controller. Task drop cancels;
//! the timer holds only a weak controller reference and cannot keep a screen alive.
use super::{Configuration, Content, Options, Place, Tip};
use gpui::{App, Task, Window};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
struct Pending {
    place: Place,
    token: Rc<()>,
    tip: Option<Tip>,
    // Ownership is cancellation: GPUI Task::drop cancels rather than detaching.
    _task: Task<()>,
}
#[derive(Debug, Default)]
pub(super) struct State {
    pub(super) shown: Option<Tip>,
    pending: Option<Pending>,
}
impl State {
    pub(super) fn refresh_trigger(
        &mut self,
        place: &Place,
        trigger: &Rc<()>,
        content: &Content,
        configuration: Configuration,
        options: Options,
    ) {
        if let Some(pending) = &mut self.pending
            && Rc::ptr_eq(&pending.place, place)
            && let Some(tip) = &mut pending.tip
        {
            tip.trigger = Rc::downgrade(trigger);
            tip.content = content.clone();
            tip.configuration = configuration;
            tip.options = options;
        }
        if let Some(tip) = &mut self.shown
            && Rc::ptr_eq(&tip.place, place)
        {
            tip.trigger = Rc::downgrade(trigger);
            tip.content = content.clone();
            tip.configuration = configuration;
            tip.options = options;
        }
    }
    pub(super) fn dismiss(&mut self) {
        self.shown = None;
        self.pending = None;
    }
    pub(super) fn leave(&mut self, place: &Place) {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| Rc::ptr_eq(&pending.place, place))
        {
            self.pending = None;
        }
        if self
            .shown
            .as_ref()
            .is_some_and(|tip| Rc::ptr_eq(&tip.place, place))
        {
            self.shown = None;
        }
    }
    fn publish(&mut self, token: &Rc<()>) {
        if let Some(pending) = &mut self.pending
            && Rc::ptr_eq(&pending.token, token)
        {
            // An unmounted trigger may never receive a final hover-leave event.
            // Its old listener must not survive solely because this timer does.
            self.shown = pending
                .tip
                .take()
                .filter(|tip| tip.trigger.upgrade().is_some());
        }
    }
}
pub(super) fn entered(state: &Rc<RefCell<State>>, tip: Tip, window: &mut Window, cx: &App) {
    state.borrow_mut().dismiss();
    let delay = tip.configuration.delay();
    if delay.is_zero() {
        state.borrow_mut().shown = Some(tip);
    } else {
        let weak = Rc::downgrade(state);
        let token = Rc::new(());
        let delivered_token = Rc::clone(&token);
        let handle = window.window_handle();
        let task = cx.spawn(async move |cx| {
            cx.background_executor().timer(delay).await;
            let _closed = handle.update(cx, |_, window, _cx| {
                if let Some(state) = weak.upgrade() {
                    state.borrow_mut().publish(&delivered_token);
                    window.refresh();
                }
            });
        });
        state.borrow_mut().pending = Some(Pending {
            place: Rc::clone(&tip.place),
            token,
            tip: Some(tip),
            _task: task,
        });
    }
    window.refresh();
}
