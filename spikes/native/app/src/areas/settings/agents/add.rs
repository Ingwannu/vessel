//! Record-only provider form; page tickets and validation remain in view.
use crate::shell::Shell;
use std::{cell::Cell, rc::Rc};
use vessel_app_kit::{
    DialogState, TextField,
    ui::{AnyElement, App, Context, Window},
};
use vessel_app_view::agent_status::{
    addition::{Intent, Refused},
    pages::MachineKey,
};
use vessel_proto::{RequestId, agents::CatalogueEntry};
mod actions;
mod draw;
pub(super) use draw::{dialog, notice};
#[derive(Clone, Copy)]
enum Phase {
    Editing,
    Sending(RequestId),
}
#[derive(Clone, Copy)]
pub(super) enum Settlement {
    Accepted,
    Failed,
}
#[derive(Clone, Copy)]
enum Restore {
    Keep,
    Dismiss,
}
pub(super) struct State {
    dialog: DialogState,
    form: Option<Form>,
    refused: Option<Refused>,
    restore: Cell<Restore>,
}
struct Form {
    intent: Rc<Intent>,
    machine: MachineKey,
    registry: Option<CatalogueEntry>,
    name: TextField,
    program: TextField,
    arguments: TextField,
    phase: Phase,
    invalid: Option<Refused>,
}
impl State {
    pub(super) fn new(cx: &mut App) -> Self {
        Self {
            dialog: DialogState::new(cx),
            form: None,
            refused: None,
            restore: Cell::new(Restore::Keep),
        }
    }
    pub(super) fn close_later(&mut self) {
        if self.form.take().is_some() {
            self.restore.set(Restore::Dismiss);
        }
        self.refused = None;
    }
    pub(super) fn is_open(&self) -> bool {
        self.form.is_some()
    }
}
// Submitting requires the actual Save button's captured form identity. A
// delayed Enter from a discarded input must never submit a newer form.
pub(super) fn edit_only(
    _shell: &mut Shell,
    _text: String,
    _window: &mut Window,
    _cx: &mut Context<'_, Shell>,
) {
}
pub(super) fn changed(_shell: &mut Shell, _text: String, cx: &mut Context<'_, Shell>) {
    cx.notify();
}
