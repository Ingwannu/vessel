//! Field construction shares the existing event wiring. Keeping factories
//! here lets the public control grow captured callbacks while its listed root
//! shrinks; no input engine, Enter/IME rule or focus behavior is duplicated.
use super::{
    COMPOSER_LINES, FORM_LINES, FORM_MIN_LINES, Handlers, InputState, Kind, Known, MultilineStyle,
    SendOn, TextField, TextareaState, listen, register,
};
use gpui::{AppContext as _, Context, Window};
use std::{cell::Cell, rc::Rc};

pub(super) fn multiline<V: 'static>(
    style: MultilineStyle,
    placeholder: &str,
    on_submit: impl Fn(&mut V, String, &mut Window, &mut Context<'_, V>) + 'static,
    window: &mut Window,
    cx: &mut Context<'_, V>,
) -> TextField {
    let placeholder = placeholder.to_owned();
    let minimum = match style {
        MultilineStyle::Composer => 1,
        MultilineStyle::Form => FORM_MIN_LINES,
    };
    let maximum = match style {
        MultilineStyle::Composer => COMPOSER_LINES,
        MultilineStyle::Form => FORM_LINES,
    };
    let state = cx.new(|cx| {
        TextareaState::new(window, cx)
            .auto_grow(minimum, maximum)
            .submit_on_enter(true)
            .placeholder(placeholder)
    });
    let focused = Rc::new(Cell::new(false));
    let shortcut = match style {
        MultilineStyle::Composer => SendOn::Enter,
        MultilineStyle::Form => SendOn::ModEnter,
    };
    let send = Rc::new(Cell::new(shortcut));
    listen(
        &state,
        Handlers {
            on_submit: Box::new(on_submit),
            // A growing form changes the surrounding dialog's layout
            // and submit availability; its owner must see each edit.
            on_change: match style {
                MultilineStyle::Composer => None,
                MultilineStyle::Form => Some(|_, _, cx| cx.notify()),
            },
            focused: Rc::clone(&focused),
            send: Some(Rc::clone(&send)),
        },
        window,
        cx,
    );
    register(Known::Lines(state.downgrade()));
    TextField {
        kind: Kind::Lines(state, style),
        focused,
        send,
    }
}

pub(super) fn line<V: 'static>(
    placeholder: &str,
    on_submit: impl Fn(&mut V, String, &mut Window, &mut Context<'_, V>) + 'static,
    on_change: Option<fn(&mut V, String, &mut Context<'_, V>)>,
    window: &mut Window,
    cx: &mut Context<'_, V>,
) -> TextField {
    let placeholder = placeholder.to_owned();
    let state = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
    let focused = Rc::new(Cell::new(false));
    listen(
        &state,
        Handlers {
            on_submit: Box::new(on_submit),
            on_change,
            focused: Rc::clone(&focused),
            send: None,
        },
        window,
        cx,
    );
    register(Known::Line(state.downgrade()));
    TextField {
        kind: Kind::Line(state),
        focused,
        send: Rc::new(Cell::new(SendOn::Enter)),
    }
}
