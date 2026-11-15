//! Settings values use the same row frame as other facts. A value that can
//! be copied draws its copy button at every width.
use vessel_app_kit::copy::{CopyLook, copy_button};
use vessel_app_kit::tooltip::{Place, Tooltips};
use vessel_app_kit::ui::{
    App, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _, ParentElement as _,
    Stateful, Styled, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, fonts, settings, theme::spacing};
#[derive(Debug, Clone, Copy)]
pub(super) enum Value<'a> {
    Code(&'a str),
    /// A path: its own element of a set width, cut in the middle when it is
    /// longer, the whole of it in a tooltip (the copy button copies the
    /// whole).
    Path(&'a str),
    Text(&'a str),
}
pub(super) struct Fact<'a> {
    pub(super) id: &'static str,
    pub(super) label: &'a str,
    pub(super) value: Value<'a>,
    pub(super) first: bool,
}
/// Where a cut path's tooltip is kept and placed.
pub(super) struct Tip<'a> {
    pub(super) tips: &'a Tooltips,
    pub(super) place: &'a Place,
}
/// The widest a fact's control is on a pointer's width (the row's words
/// take the rest).
const CONTROL: f32 = 420.0;
/// The copy button's side.
const fn copy_size(compact: bool) -> f32 {
    if compact {
        vessel_app_kit::breakpoint::TOUCH_TARGET
    } else {
        32.0
    }
}
/// A path's set width: the control's, less the copy button and its gap; on
/// a phone the row's content width (the control sits under the row's words
/// there).
fn path_width(window: &Window, compact: bool) -> f32 {
    let room = if compact {
        settings::PHONE_ROW_INSET.mul_add(-1.0, f32::from(window.viewport_size().width))
    } else {
        CONTROL
    };
    spacing::S2
        .mul_add(-1.0, copy_size(compact).mul_add(-1.0, room))
        .max(0.0)
}
pub(super) fn row(
    fact: &Fact<'_>,
    tip: Option<&Tip<'_>>,
    tokens: &Tokens,
    window: &Window,
    cx: &App,
) -> Div {
    let compact = vessel_app_kit::breakpoint::phone(window);
    let control = match fact.value {
        Value::Code(text) => copyable(
            fact.id,
            super::widgets::inline_value(fact.id, text, tokens, window),
            text,
            compact,
            tokens,
            window,
            cx,
        ),
        Value::Path(path) => {
            let width = path_width(window, compact);
            let shown = fonts::elide_middle(window, path, Type::Mono, width);
            let cut = shown != path;
            let value = div()
                .id("path")
                .w(px(width))
                .flex_none()
                .flex()
                // Under its words on a phone, as the other values are; at
                // the row's end beside them on a pointer's width.
                .when(compact, Styled::justify_start)
                .when(!compact, Styled::justify_end)
                .type_(Type::Mono)
                .whitespace_nowrap()
                .text_color(tokens.neutral.foreground)
                .debug_selector(move || format!("{}-content", fact.id))
                .child(shown);
            let value = match tip {
                Some(tip) if cut => tip.tips.on(value, tip.place, path.to_owned(), compact),
                _ => value,
            };
            copyable(fact.id, value, path, compact, tokens, window, cx)
        }
        Value::Text(text) => div()
            .id(fact.id)
            .max_w(px(CONTROL))
            .type_(Type::Body)
            .text_color(tokens.neutral.foreground_muted)
            .child(text.to_owned()),
    }
    // A wrapped compact control still has to fit its row's content box.
    .when(compact, |control| control.min_w_0().max_w_full());
    settings::row_at(
        fact.label.to_owned(),
        None,
        None,
        Some(control.into_any_element()),
        fact.first,
        compact,
        tokens,
    )
}
/// A value with its copy button, always drawn: a button shown only on
/// hover kept its room, so the value ended 40 points short of the rows
/// beside it (the coordinator's review); the command boxes and a phone's
/// rows draw it too.
fn copyable(
    id: &'static str,
    value: Stateful<Div>,
    text: &str,
    compact: bool,
    tokens: &Tokens,
    window: &Window,
    cx: &App,
) -> Stateful<Div> {
    div()
        .id(id)
        .on_scroll(|_, _, cx| cx.stop_propagation())
        .max_w(px(CONTROL))
        .flex()
        .items_center()
        .gap(px(spacing::S2))
        .child(value)
        .child(
            copy_button(
                "copy",
                text.to_owned().into(),
                CopyLook {
                    glyph: tokens.neutral.foreground_muted,
                    hover: tokens.neutral.surface2,
                },
                window,
                cx,
            )
            .size(px(copy_size(compact)))
            .flex_none()
            .debug_selector(move || format!("{id}-copy")),
        )
}

#[cfg(test)]
mod tests;
