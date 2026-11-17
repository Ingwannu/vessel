//! Label values choose generic theme identities; one swatch row picks them.
//!
//! [Decision Log]
//! - Purpose: Paseo's label colour picker (`workspace-labels/swatch.tsx`
//!   `WorkspaceLabelSwatchRow`, used by the manager's edit view and the
//!   picker's create view): one row of the ten colours as 20-point circles,
//!   gap 12, wrapping; the selection an inset 2-point `foreground` ring that
//!   cannot reflow the row; the colour names only as accessibility labels.
//! - Before: named Ghost pills with a dot and a fading selection border (the
//!   coordinator's RETURN on the manager body, 2026-11-16).
//! - Choice: a plain circle per colour drawn here, GPUI's border sitting
//!   inside the box as React Native's does. No animation (Paseo's ring
//!   switches at once). Read-only rows draw the same circles without a handler.
//! - Impact: the colour names have no place to go until the kit has an
//!   accessibility tree (the kit-wide gap `host_rename.rs` records); Paseo's
//!   12-point `hitSlop` is not reproduced (its web build has none).

use vessel_app_kit::theme::{
    identity::{self, Hue},
    spacing::{S1_5, S3},
};
use vessel_app_kit::ui::{
    Context, Div, ElementId, FluentBuilder as _, InteractiveElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, dot};
use vessel_app_view::labels::{Color, EditAccess, HostState, Label};

use crate::shell::Shell;

/// `DOT_SIZE`: the mark in front of a label's name.
const DOT_SIZE: f32 = 10.0;
/// `SWATCH_SIZE`: the swatch row's only unit.
pub(super) const SWATCH_SIZE: f32 = 20.0;
pub(super) fn foreground(color: Color, tokens: &Tokens) -> gpui::Hsla {
    identity::foreground(hue(color), tokens.scheme)
}

/// A label where it is read, not managed (Paseo's `WorkspaceLabelChip`):
/// its name in its colour on a 10% ground of the same hue, a pill exactly
/// as tall as its 12/16 line (no vertical padding, so a row does not grow
/// for it), the ground 6 points past the name; a long name is cut.
pub(crate) fn chip(label: &Label, tokens: &Tokens) -> Div {
    let hue = hue(label.color);
    div()
        .flex_shrink(1.0)
        .min_w_0()
        .px(px(S1_5))
        .rounded_full()
        .bg(identity::tint(hue))
        .child(
            div()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .type_(Type::Meta)
                .text_color(identity::foreground(hue, tokens.scheme))
                .child(label.name.as_str().to_owned()),
        )
}

pub(super) fn mark(color: Color, tokens: &Tokens) -> Div {
    dot(foreground(color, tokens)).size(px(DOT_SIZE))
}

pub(super) fn choices(
    shell: &Shell,
    selected: Color,
    target: &super::actions::Target,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let target = target.clone();
    let on_choose: ColorChoice = std::rc::Rc::new(move |shell, color, cx| {
        if access(shell) == EditAccess::ReadOnly {
            return;
        }
        let state = &mut shell.areas.labels;
        if !target.matches(
            state.opening,
            &state.manager,
            state
                .selected_host()
                .and_then(|host| host.connection.as_ref()),
        ) {
            return;
        }
        if let Some(draft) = state.manager.draft_mut()
            && draft.color() != color
        {
            draft.set_color(color);
            cx.notify();
        }
    });
    choices_for(shell, selected, access(shell), &on_choose, tokens, cx)
}

pub(super) type ColorChoice = std::rc::Rc<dyn Fn(&mut Shell, Color, &mut Context<'_, Shell>)>;

/// Both label workflows use this one swatch row (Paseo's
/// `WorkspaceLabelSwatchRow`).
pub(super) fn choices_for(
    shell: &Shell,
    selected: Color,
    editability: EditAccess,
    on_choose: &ColorChoice,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    div()
        .debug_selector(|| "label-swatches".to_owned())
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(px(S3))
        .children(Color::ALL.into_iter().map(|color| {
            let on_choose = std::rc::Rc::clone(on_choose);
            div()
                .id(ElementId::Name(format!("label-swatch-{color:?}").into()))
                .debug_selector(move || format!("label-swatch-{color:?}"))
                // Paseo's swatch: a radio named by its colour, checked when
                // chosen (`swatch.tsx` 99-104).
                .role(gpui::Role::RadioButton)
                .aria_label(color_name(shell, color))
                .aria_toggled(if color == selected {
                    gpui::Toggled::True
                } else {
                    gpui::Toggled::False
                })
                .flex_none()
                .size(px(SWATCH_SIZE))
                .rounded_full()
                .bg(foreground(color, tokens))
                .when(color == selected, |swatch| {
                    swatch.border_2().border_color(tokens.neutral.foreground)
                })
                .when(editability == EditAccess::Editable, |swatch| {
                    swatch
                        .cursor_pointer()
                        .on_click(cx.listener(move |shell, _, _window, cx| {
                            on_choose(shell, color, cx);
                        }))
                })
        }))
}

/// The colour's name (Paseo's `workspaceLabelColorName`).
fn color_name(shell: &Shell, color: Color) -> String {
    let words = shell.words.labels();
    match color {
        Color::Violet => words.color_violet(),
        Color::Sky => words.color_sky(),
        Color::Emerald => words.color_emerald(),
        Color::Orange => words.color_orange(),
        Color::Pink => words.color_pink(),
        Color::Indigo => words.color_indigo(),
        Color::Teal => words.color_teal(),
        Color::Red => words.color_red(),
        Color::Amber => words.color_amber(),
        Color::Blue => words.color_blue(),
    }
    .to_owned()
}

fn access(shell: &Shell) -> EditAccess {
    let state = &shell.areas.labels;
    let host = state.selected_host();
    state
        .manager
        .edit_access(host.map_or(HostState::Offline, |host| host.state))
}

const fn hue(color: Color) -> Hue {
    match color {
        Color::Violet => Hue::Violet,
        Color::Sky => Hue::Sky,
        Color::Emerald => Hue::Emerald,
        Color::Orange => Hue::Orange,
        Color::Pink => Hue::Pink,
        Color::Indigo => Hue::Indigo,
        Color::Teal => Hue::Teal,
        Color::Red => Hue::Red,
        Color::Amber => Hue::Amber,
        Color::Blue => Hue::Blue,
    }
}
