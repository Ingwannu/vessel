//! Label values choose generic theme identities; existing buttons draw choices.

use gpui::AnimationExt as _;
use vessel_app_kit::motion::{QUICK, transition};
use vessel_app_kit::theme::{
    identity::{self, Hue},
    spacing::S2,
};
use vessel_app_kit::ui::{
    Context, Div, ElementId, InteractiveElement as _, IntoElement as _, ParentElement as _,
    Styled as _, div, px,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, button_with_kind, dot};
use vessel_app_view::labels::{Color, EditAccess, HostState};

use crate::shell::Shell;

/// Reference menu-dot size, distinct from the existing button's touch target.
const DOT_SIZE: f32 = 10.0;

/// Visual identities never wrap back to an older animation key.
#[derive(Default)]
pub(super) struct ColorTurn(u64);

impl ColorTurn {
    fn advance(&mut self) {
        self.0 = self.0.saturating_add(1);
    }
    fn key(&self, color: Color) -> ElementId {
        ElementId::Name(format!("label-color-{color:?}-{}", self.0).into())
    }
}

pub(super) fn mark(color: Color, tokens: &Tokens) -> Div {
    dot(identity::foreground(hue(color), tokens.scheme)).size(px(DOT_SIZE))
}

pub(super) fn choices(
    shell: &Shell,
    selected: Color,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    div()
        .flex()
        .flex_wrap()
        .gap(px(S2))
        .children(Color::ALL.into_iter().map(|color| {
            let button = button_with_kind(
                ElementId::Name(format!("label-swatch-{color:?}").into()),
                name(shell, color),
                ButtonKind::Ghost,
                match access(shell) {
                    EditAccess::Editable => ButtonState::Enabled,
                    EditAccess::ReadOnly => ButtonState::Disabled,
                },
                tokens,
            )
            .min_h(px(vessel_app_kit::breakpoint::TOUCH_TARGET))
            .border_1()
            .border_color(if color == selected {
                tokens.neutral.foreground
            } else {
                tokens.neutral.border
            })
            .gap(px(S2))
            .child(mark(color, tokens))
            .on_click(cx.listener(move |shell, _, _window, cx| {
                if access(shell) == EditAccess::ReadOnly {
                    return;
                }
                let state = &mut shell.areas.labels;
                if let Some(draft) = state.manager.draft_mut() {
                    if draft.color() != color {
                        draft.set_color(color);
                        state.color_turn.advance();
                        cx.notify();
                    }
                }
            }));
            if color == selected {
                button
                    .with_animation(
                        shell.areas.labels.color_turn.key(color),
                        transition(QUICK),
                        vessel_app_kit::ui::Styled::opacity,
                    )
                    .into_any_element()
            } else {
                button.into_any_element()
            }
        }))
}

fn access(shell: &Shell) -> EditAccess {
    let state = &shell.areas.labels;
    let host = state
        .manager
        .selected()
        .and_then(|key| state.hosts.iter().find(|host| &host.key == key));
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

fn name(shell: &Shell, color: Color) -> &'static str {
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
}
