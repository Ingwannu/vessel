//! Opt-in context requests use the existing editor and retain only weak input identity.

use super::{Kind, TextField};
use gpui::{App, Entity, FocusHandle, Focusable as _, Pixels, Point, WeakFocusHandle, Window};
use gpui_base::input::{InputBaseState, InputContextMenuCapabilities, InputModeKind};
use std::rc::Rc;

/// Whether the original editor currently permits copying its selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyPermission {
    Available,
    Unavailable,
}

/// Whether the original editor currently permits pasting text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PastePermission {
    Available,
    Unavailable,
}

/// Editing facts come from the existing input engine, including masked values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Editing {
    Editable,
    ReadOnly,
    Disabled,
}

/// No selection text or clipboard contents cross the host boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Facts {
    pub copy: CopyPermission,
    pub paste: PastePermission,
    pub editing: Editing,
}

impl From<InputContextMenuCapabilities> for Facts {
    fn from(value: InputContextMenuCapabilities) -> Self {
        Self {
            copy: if !value.is_disabled() && value.is_copyable() {
                CopyPermission::Available
            } else {
                CopyPermission::Unavailable
            },
            paste: if value.is_editable() {
                PastePermission::Available
            } else {
                PastePermission::Unavailable
            },
            editing: if value.is_disabled() {
                Editing::Disabled
            } else if value.is_readonly() {
                Editing::ReadOnly
            } else {
                Editing::Editable
            },
        }
    }
}

/// A fresh UI probe, never host/session authority or a second editor state.
pub type Probe = Rc<dyn Fn(&App) -> Option<Facts>>;
/// The host owns menu presentation and rechecks this request before delivery.
pub type Handler = Rc<dyn Fn(Request, &mut Window, &mut App)>;
type SelectionFocus = Rc<dyn Fn(Option<FocusHandle>, &mut App)>;

/// Original focus and a weak live fact probe survive deferred host delivery.
#[derive(Clone)]
pub struct Request {
    pub position: Point<Pixels>,
    original: WeakFocusHandle,
    probe: Probe,
    selection: Option<SelectionFocus>,
}

impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Request")
            .field("position", &self.position)
            .finish_non_exhaustive()
    }
}

impl Request {
    /// Other editor owners can supply their own weak fact probe. This grants
    /// only UI identity; domain admission remains with the host callback.
    #[must_use]
    pub fn new(position: Point<Pixels>, original: WeakFocusHandle, probe: Probe) -> Self {
        Self {
            position,
            original,
            probe,
            selection: None,
        }
    }

    /// Read current permissions rather than the snapshot from mouse-down.
    #[must_use]
    pub fn facts(&self, cx: &App) -> Option<Facts> {
        (self.probe)(cx)
    }

    /// A retained request must not redirect a popup into another focused editor.
    #[must_use]
    pub fn current(&self, window: &Window, cx: &App) -> bool {
        self.original
            .upgrade()
            .is_some_and(|focus| focus.is_focused(window))
            && self
                .facts(cx)
                .is_some_and(|facts| facts.editing != Editing::Disabled)
    }

    /// A host that deferred again calls this only after its actual popup opens.
    /// Keep selection highlighted through the editor's existing weak focus API.
    pub fn retain_selection(&self, window: &Window, cx: &mut App) {
        if let Some(selection) = &self.selection {
            let menu = window
                .focused(cx)
                .filter(|focus| Some(focus) != self.original.upgrade().as_ref());
            selection(menu, cx);
        }
    }
}

impl TextField {
    /// Opt this field into the host's existing menu. Unconfigured fields retain
    /// their current engine callback; no global factory/default is changed.
    pub fn set_context_menu_handler(&self, handler: Handler, cx: &mut App) {
        match &self.kind {
            Kind::Line(state) => install(state, handler, cx),
            Kind::Lines(state, _) => install(state, handler, cx),
        }
    }
}

fn install<M: InputModeKind>(state: &Entity<InputBaseState<M>>, handler: Handler, cx: &mut App) {
    let original = state.read(cx).focus_handle(cx).downgrade();
    let input = state.downgrade();
    let probe: Probe = Rc::new(move |cx| {
        input
            .upgrade()
            .map(|input| input.read(cx).context_menu_capabilities().into())
    });
    let input = state.downgrade();
    let selection: SelectionFocus = Rc::new(move |focus, cx| {
        if let Some(input) = input.upgrade() {
            input.update(cx, |input, cx| input.set_selection_focus(focus, cx));
        }
    });
    state.update(cx, |state, _cx| {
        state.on_context_menu(Rc::new(move |_native, _snapshot, position, window, cx| {
            // gpui-base defers this callback after releasing its input. The
            // selection/mask/editability may have changed since mouse-down.
            let mut request = Request::new(position, original.clone(), Rc::clone(&probe));
            request.selection = Some(Rc::clone(&selection));
            if request.current(window, cx) {
                handler(request, window, cx);
            }
        }));
    });
}

#[cfg(test)]
mod tests;
