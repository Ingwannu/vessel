//! A terminal tab's panel, as Paseo's `TerminalPane` draws it
//! (`components/terminal-pane.tsx` 1162-1261 and its styles): "Host is not
//! connected" centred while the machine is away; else the screen edge to
//! edge on the terminal's background, a spinner over it while it attaches
//! (a 16 % black scrim), and a red line under it when it cannot be live
//! (border on top, `surface1`, two lines at most). The close confirm is
//! Paseo's `confirmDialog` for a terminal (destructive Close).

use gpui::{ElementInputHandler, KeyDownEvent, ScrollEvent};
use vessel_app_kit::loading::{Size as SpinnerSize, activity_indicator};
use vessel_app_kit::picker::anchor_probe;
use vessel_app_kit::terminal_grid::cell_size;
use vessel_app_kit::theme::spacing::{S1, S3, S4};
use vessel_app_kit::ui::{
    AnyElement, Context, ElementId, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    MouseButton, ParentElement as _, Pixels, StatefulInteractiveElement as _, Styled as _, Window,
    canvas, div, point, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, DialogContent, DialogSize, Tokens, Type, TypeExt as _, breakpoint,
    button_with_kind, cached_view, dialog,
};
use vessel_app_view::layout::Target;
use vessel_app_view::terminal_tabs::Trouble;

use super::{PaneKey, Stage, pane_key};
use crate::areas::layout::registry::PanelContext;
use crate::areas::terminal::convert;
use crate::areas::terminal::font_size;
use crate::areas::terminal::scroll::Step;
use crate::shell::Shell;

/// Paseo's `attachOverlay` scrim: black at 16 %.
const SCRIM_OPACITY: f32 = 0.16;

/// A terminal tab's body.
pub(crate) fn panel(
    shell: &Shell,
    panel: &PanelContext<'_>,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let Target::Terminal(terminal) = panel.target else {
        return div().into_any_element();
    };
    let Some(key) = pane_key(panel.workspace, *terminal) else {
        return centred(shell.words.layout().panel_unavailable(), tokens);
    };
    let words = shell.words.terminal_tabs();
    if !shell.machine_online(&key.machine) {
        return centred(words.host_disconnected(), tokens);
    }
    let background = convert::hsla(convert::palette(&tokens.terminal).background);
    let tabs = &shell.areas.terminal_tabs;
    let Some(pane) = tabs.pane(&key).filter(|pane| !pane.observers.is_empty()) else {
        // The pane (and its follow) is made on the next frame.
        let ensure = key;
        cx.on_next_frame(window, move |shell, window, cx| {
            shell.terminal_tab_ensure(&ensure, window, cx);
        });
        return surface(background)
            .child(attaching(tokens))
            .into_any_element();
    };
    let cell = cell_size(window, font_size());
    let measured = key.clone();
    cx.on_next_frame(window, move |shell, _window, cx| {
        shell.terminal_tab_measured(&measured, cell, cx);
    });
    if tabs.focus_next.as_ref() == Some(&key) {
        let focused = key.clone();
        cx.on_next_frame(window, move |shell, window, cx| {
            shell.terminal_tab_focus(&focused, window, cx);
        });
    }
    let keys = pane.keys.clone();
    let focus = pane.focus.clone();
    let input = canvas(
        |_bounds, _window, _cx| {},
        move |bounds, (), window, cx| {
            window.handle_input(&focus, ElementInputHandler::new(bounds, keys), cx);
        },
    )
    .absolute()
    .size_full();
    let pressed = key.clone();
    let typed = key.clone();
    let scrolled = key.clone();
    let screen = surface(background)
        .id(ElementId::Name(
            format!("terminal-surface-{}", key.terminal).into(),
        ))
        .track_focus(&pane.focus)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |shell, _event, window, cx| {
                shell.terminal_tab_focus(&pressed, window, cx);
            }),
        )
        .on_key_down(
            cx.listener(move |shell, event: &KeyDownEvent, _window, cx| {
                shell.terminal_tab_key(&typed, event, cx);
            }),
        )
        .on_scroll(cx.listener(move |shell, event: &ScrollEvent, _window, cx| {
            shell.terminal_tab_scroll(&scrolled, event, cell, cx);
        }))
        .child(cached_view(&pane.grid))
        .child(anchor_probe(&pane.probe))
        .child(input)
        .children(preedit(shell, &key, pane.preedit.as_deref(), tokens, cx))
        .when(pane.stage == Stage::Attaching, |screen| {
            screen.child(attaching(tokens))
        });
    div()
        .size_full()
        .min_h_0()
        .flex()
        .flex_col()
        .bg(tokens.neutral.surface0)
        .child(screen)
        .children(trouble_line(shell, pane.stage, tokens))
        .into_any_element()
}

/// Paseo's `errorRow`: why the screen is not live, in red, two lines at
/// most, under a rule.
fn trouble_line(shell: &Shell, stage: Stage, tokens: &Tokens) -> Option<AnyElement> {
    let words = shell.words.terminal_tabs();
    let said = match stage {
        Stage::Trouble(Trouble::Exited) => words.exited(),
        Stage::Trouble(Trouble::NotFound) => words.not_found(),
        Stage::Trouble(Trouble::Unsubscribable) => words.unable_to_subscribe(),
        Stage::Offline => words.host_disconnected(),
        Stage::Attaching | Stage::Live => return None,
    };
    Some(
        div()
            .flex_none()
            .px(px(S3))
            .py(px(S1))
            .border_t_1()
            .border_color(tokens.neutral.border)
            .bg(tokens.neutral.surface1)
            .child(
                div()
                    .type_(Type::Meta)
                    .text_color(tokens.neutral.destructive)
                    .line_clamp(2)
                    .child(said.to_owned()),
            )
            .into_any_element(),
    )
}

/// The terminal's own surface, edge to edge.
fn surface(background: gpui::Hsla) -> gpui::Div {
    div()
        .flex_1()
        .min_h_0()
        .relative()
        .overflow_hidden()
        .bg(background)
}

/// Paseo's `centerState`: one muted line in the middle.
fn centred(said: &str, tokens: &Tokens) -> AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .px(px(S4))
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground_muted)
                .text_center()
                .child(said.to_owned()),
        )
        .into_any_element()
}

/// Paseo's `terminal-attach-loading`: a small spinner over a light scrim.
fn attaching(tokens: &Tokens) -> AnyElement {
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(gpui::black().opacity(SCRIM_OPACITY))
        .child(activity_indicator(
            "terminal-attach-loading",
            SpinnerSize::Small,
            tokens.neutral.foreground_muted,
        ))
        .into_any_element()
}

/// The IME's composing text, at the cursor's cell, as xterm shows its
/// composition view there.
fn preedit(
    shell: &Shell,
    key: &PaneKey,
    text: Option<&str>,
    tokens: &Tokens,
    cx: &Context<'_, Shell>,
) -> Option<AnyElement> {
    let text = text?;
    let pane = shell.areas.terminal_tabs.pane(key)?;
    // The cursor's cell inside the pane's own box (its origin at zero).
    let room = gpui::Bounds {
        origin: point(Pixels::ZERO, Pixels::ZERO),
        size: pane.probe.get()?.size,
    };
    let at = shell.terminal_tab_cursor(key, room, cx)?;
    let palette = convert::palette(&tokens.terminal);
    Some(
        div()
            .absolute()
            .left(at.origin.x)
            .top(at.origin.y)
            .h(at.size.height)
            .bg(convert::hsla(palette.background))
            .text_color(convert::hsla(palette.foreground))
            .underline()
            .text_size(font_size())
            .child(text.to_owned())
            .into_any_element(),
    )
}

/// The close confirm, when one is asked.
pub(crate) fn overlay(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    if shell.areas.terminal_tabs.listed.is_some() {
        cx.on_next_frame(window, |shell, window, cx| {
            shell.terminal_tabs_reconcile(crate::areas::layout::mode(window), cx);
        });
    }
    shell.areas.terminal_tabs.confirm.as_ref()?;
    let words = shell.words.terminal_tabs();
    let narrow = breakpoint::phone(window);
    let footer = vec![
        button_with_kind(
            "terminal-close-cancel",
            words.cancel().to_owned(),
            ButtonKind::Secondary,
            ButtonState::Enabled,
            tokens,
        )
        .on_click(cx.listener(|shell, _event, window, cx| shell.terminal_tab_keep(window, cx)))
        .into_any_element(),
        button_with_kind(
            "terminal-close-confirm",
            words.close().to_owned(),
            ButtonKind::Destructive,
            ButtonState::Enabled,
            tokens,
        )
        .on_click(cx.listener(|shell, _event, window, cx| shell.terminal_tab_close(window, cx)))
        .into_any_element(),
    ];
    Some(dialog(
        &shell.areas.terminal_tabs.dialog,
        DialogContent {
            title: words.close_title().to_owned().into(),
            body: div()
                .type_(Type::Body)
                .child(words.close_message().to_owned())
                .into_any_element(),
            footer,
            size: DialogSize::Normal,
        },
        tokens,
        window,
        narrow,
        Shell::terminal_tab_keep,
        cx,
    ))
}

impl Shell {
    /// Make `key`'s pane (an attach, unless this app opened it), watch its
    /// focus, and follow its terminal.
    pub(super) fn terminal_tab_ensure(
        &mut self,
        key: &PaneKey,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let tabs = &mut self.areas.terminal_tabs;
        let fresh = !tabs.panes.contains_key(key);
        if fresh {
            let pane = super::Pane::new(vessel_app_view::terminal::Origin::Attached, cx, key);
            let _none = self.areas.terminal_tabs.panes.insert(key.clone(), pane);
        }
        let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key) else {
            return;
        };
        if pane.observers.is_empty() {
            let focus = pane.focus.clone();
            let gained = key.clone();
            let lost = key.clone();
            pane.observers = vec![
                cx.on_focus_in(&focus, window, move |shell, window, cx| {
                    shell.terminal_tab_focused_in(&gained, window, cx);
                }),
                cx.on_focus_out(&focus, window, move |shell, _event, _window, cx| {
                    shell.terminal_tab_focused_out(&lost, cx);
                }),
            ];
        }
        if fresh {
            self.terminal_tab_follow(key);
        }
        cx.notify();
    }

    /// Give `key`'s pane the keyboard.
    pub(super) fn terminal_tab_focus(
        &mut self,
        key: &PaneKey,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self.areas.terminal_tabs.focus_next.as_ref() == Some(key) {
            self.areas.terminal_tabs.focus_next = None;
        }
        if let Some(pane) = self.areas.terminal_tabs.panes.get(key) {
            pane.focus.focus(window, cx);
        }
    }

    fn terminal_tab_focused_in(
        &mut self,
        key: &PaneKey,
        _window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.areas.terminal_tabs.focused = Some(key.clone());
        self.terminal_tab_claim(key);
        self.terminal_tab_blink(cx);
        cx.notify();
    }

    fn terminal_tab_focused_out(&mut self, key: &PaneKey, cx: &mut Context<'_, Self>) {
        if self.areas.terminal_tabs.focused.as_ref() == Some(key) {
            self.areas.terminal_tabs.focused = None;
        }
        self.terminal_tab_blink(cx);
        cx.notify();
    }

    /// A wheel or a touch pan over a tab's screen.
    fn terminal_tab_scroll(
        &mut self,
        key: &PaneKey,
        event: &ScrollEvent,
        cell: vessel_app_kit::terminal_grid::CellSize,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key) else {
            return;
        };
        if let Some(model) = pane.model.as_ref() {
            pane.viewport.refresh(model.grid());
        }
        let delta = event.delta.pixel_delta(cell.height).y;
        match pane.wheel.step(delta, cell) {
            Some(Step::Earlier(lines)) => pane.viewport.earlier_retained(lines),
            Some(Step::Later(lines)) => pane.viewport.later_retained(lines),
            None => return,
        }
        self.terminal_tab_draw(key, cx);
        cx.stop_propagation();
    }
}
