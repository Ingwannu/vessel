//! The virtual key bar under a shell tab at compact widths (P4), as
//! Paseo's `TerminalPane` draws it (`components/terminal-pane.tsx`
//! 1241-1257 and its `keyboard*`/`keyButton*` styles), from the view's
//! tables and sticky modifiers (`view::terminal::key_bar`).
//!
//! [Decision Log]
//! - Purpose: on a narrow window, Esc, Tab, the arrows and Ctrl/Alt/Shift
//!   chords a step away, as Paseo's desktop shows them when compact.
//! - Choice: shown on a shell tab only while the window is compact (the
//!   kit's phone width, Paseo's compact). A key goes out through the
//!   pane's one sender, as typing does. A modifier toggles and stays set
//!   until the next key, from the bar or the keyboard. Either press gives
//!   the shell its focus back.
//! - Stated differences: no keyboard toggle (a desktop has no soft
//!   keyboard) and no Paste (Paseo's desktop has neither). Their slots are
//!   left out of the rows, so the others widen, as Paseo's do when Paste is
//!   not shown.
//! - Cost: one row of buttons more under the shell when compact. The rows
//!   are picked from the window's width, the bar's own width at compact
//!   sizes, less its padding.

use gpui::FontWeight;
use vessel_app_kit::theme::radius;
use vessel_app_kit::theme::spacing::{S1, S2};
use vessel_app_kit::ui::{
    AnyElement, Context, ElementId, FluentBuilder as _, InteractiveElement as _,
    IntoElement as _, ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window,
    div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, breakpoint};
use vessel_app_view::terminal::key_bar::{self, Control, KEY_HEIGHT, MAX_WIDTH, Sticky};

use super::PaneKey;
use crate::shell::Shell;

/// Whether this surface draws `control`: the desktop app has no soft
/// keyboard to toggle and no Paste on the bar.
const fn drawn(control: Control) -> bool {
    matches!(control, Control::Key(_) | Control::Modifier(_))
}

/// The bar, when the window is compact.
pub(super) fn bar(
    sticky: Sticky,
    key: &PaneKey,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    if !breakpoint::phone(window) {
        return None;
    }
    let available = (f32::from(window.viewport_size().width) - 2.0 * S2).min(MAX_WIDTH);
    let mut rows = div()
        .flex()
        .flex_col()
        .gap(px(S1))
        .w_full()
        .max_w(px(MAX_WIDTH))
        .mx_auto();
    for row in key_bar::rows(available, true) {
        let mut line = div().flex().flex_row().items_center().gap(px(S1));
        for control in row.iter().copied().filter(|control| drawn(*control)) {
            line = line.child(button(control, sticky, key, tokens, cx));
        }
        rows = rows.child(line);
    }
    Some(
        div()
            .flex_none()
            .border_t_1()
            .border_color(tokens.neutral.border)
            .bg(tokens.neutral.surface0)
            .p(px(S2))
            .child(rows)
            .into_any_element(),
    )
}

/// One key or modifier (Paseo's `keyButton`: 34 high, a 1-point border on
/// `surface1`, `surface2` hovered; a set modifier is outlined in the
/// accent on `surface2` with the foreground text).
fn button(
    control: Control,
    sticky: Sticky,
    key: &PaneKey,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let (id, label, set) = match control {
        Control::Key(bar_key) => (bar_key.id(), bar_key.label(), false),
        Control::Modifier(modifier) => (modifier.id(), modifier.label(), sticky.is_set(modifier)),
        Control::Paste | Control::KeyboardToggle => return div().into_any_element(),
    };
    let surface2 = tokens.neutral.surface2;
    let pressed = key.clone();
    div()
        .id(ElementId::Name(
            format!("terminal-key-{id}-{}", key.terminal).into(),
        ))
        .role(gpui::Role::Button)
        .aria_label(label.to_owned())
        // Paseo's `accessibilityState={{ selected }}` on a modifier.
        .when(matches!(control, Control::Modifier(_)), |button| {
            button.aria_selected(set)
        })
        .flex_1()
        .min_w_0()
        .h(px(KEY_HEIGHT))
        .px(px(S1))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(radius::MD))
        .border_1()
        .border_color(if set {
            tokens.neutral.accent
        } else {
            tokens.neutral.border
        })
        .bg(if set { surface2 } else { tokens.neutral.surface1 })
        .hover(move |style| style.bg(surface2))
        .cursor_pointer()
        .child(
            div()
                .type_(Type::Body)
                .font_weight(FontWeight::MEDIUM)
                .text_color(if set {
                    tokens.neutral.foreground
                } else {
                    tokens.neutral.foreground_muted
                })
                .child(label),
        )
        .on_click(cx.listener(move |shell, _event, window, cx| {
            shell.terminal_key_bar_press(&pressed, control, window, cx);
        }))
        .into_any_element()
}

impl Shell {
    /// A bar button pressed: a key goes out with the set modifiers, a
    /// modifier toggles; the shell takes its focus back.
    pub(super) fn terminal_key_bar_press(
        &mut self,
        key: &PaneKey,
        control: Control,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(pane) = self.areas.terminal_tabs.panes.get_mut(key) else {
            return;
        };
        match control {
            Control::Key(bar_key) => {
                let cursor = pane.cursor_keys();
                let sequence = key_bar::press(bar_key, &mut pane.sticky, cursor);
                self.terminal_tab_type(key, &sequence, cx);
            }
            Control::Modifier(modifier) => pane.sticky.toggle(modifier),
            Control::Paste | Control::KeyboardToggle => {}
        }
        self.terminal_tab_focus(key, window, cx);
        cx.notify();
    }

    /// Text typed or committed by the IME: with modifiers set on the bar,
    /// the next character goes out as that key with them (Paseo's
    /// `resolvePendingModifierDataInput`); else as typed.
    pub(super) fn terminal_tab_type_text(
        &mut self,
        key: &PaneKey,
        text: &str,
        cx: &mut Context<'_, Self>,
    ) {
        let resolved = self.areas.terminal_tabs.panes.get_mut(key).and_then(|pane| {
            let cursor = pane.cursor_keys();
            key_bar::typed(text, &mut pane.sticky, cursor)
        });
        match resolved {
            Some(sequence) => {
                self.terminal_tab_type(key, &sequence, cx);
                cx.notify();
            }
            None => self.terminal_tab_type(key, text, cx),
        }
    }
}
