//! Field construction shares the existing event wiring. Keeping factories
//! here lets the public control grow captured callbacks while its listed root
//! shrinks; no input engine, Enter/IME rule or focus behavior is duplicated.
use super::events::{Callbacks, Handlers, listen};
use super::{
    COMPOSER_LINES, FORM_LINES, FORM_MIN_LINES, InputState, Kind, MultilineStyle, SendOn,
    TextField, TextareaState, composition,
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
            callbacks: Callbacks::Plain {
                on_submit: Box::new(on_submit),
                // Growing forms update layout and submit availability.
                on_change: match style {
                    MultilineStyle::Composer => None,
                    MultilineStyle::Form => Some(|_, _, cx| cx.notify()),
                },
            },
            focused: Rc::clone(&focused),
            send: Some(Rc::clone(&send)),
        },
        window,
        cx,
    );
    composition::lines(&state);
    TextField {
        kind: Kind::Lines(state, style),
        focused,
        send,
    }
}

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
        send: Rc::new(Cell::new(SendOn::Enter)),
    }
}
