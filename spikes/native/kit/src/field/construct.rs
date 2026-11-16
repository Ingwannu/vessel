//! Single-line fields retain the canonical callbacks and one IME registry.
use super::events::{Callbacks, Handlers, listen};
use super::{
    FormMinimum, FormText, FormTextareaPresentation, InputState, Kind, SendOn, TextField,
    composition,
};
use gpui::{AppContext as _, Context, Window};
use std::{cell::Cell, rc::Rc};

pub(super) fn line<V: 'static>(
    placeholder: &str,
    callbacks: Callbacks<V>,
    window: &mut Window,
    cx: &mut Context<'_, V>,
) -> TextField {
    let placeholder = placeholder.to_owned();
    let state = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
    let focused = Rc::new(Cell::new(false));
    listen(
        &state,
        Handlers {
            callbacks,
            focused: Rc::clone(&focused),
            send: None,
        },
        window,
        cx,
    );
    composition::line(&state);
    TextField {
        kind: Kind::Line(state),
        focused,
        form_minimum: FormMinimum::Default,
        form_text: FormText::Body,
        form_textarea: FormTextareaPresentation::Compact,
        send: Rc::new(Cell::new(SendOn::Enter)),
    }
}
