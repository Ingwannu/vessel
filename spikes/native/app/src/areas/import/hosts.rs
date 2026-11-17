//! Paseo's host chooser (`hosts/host-chooser.tsx`) as the sheet's first page,
//! and the retirement of an opening whose drawn host metadata changed.
use super::intent::{Action, listener};
use crate::shell::Shell;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::theme::spacing::{S1, S3, S4};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, InteractiveElement as _, IntoElement as _,
    ParentElement as _, StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_view::{Fleet, Machine, connection::ConnectionTag};

/// The dialog body's 24 inset, undone: the chooser's rows reach the card's edges.
const EDGE: f32 = -24.0;

/// Paseo's `matchesHostQuery`: the label or the id contains the search.
fn matches(machine: &Machine, needle: &str) -> bool {
    needle.is_empty()
        || machine.name.to_lowercase().contains(needle)
        || machine
            .node
            .is_some_and(|node| node.to_string().contains(needle))
}

/// The connected machines, or Paseo's "No matching hosts". Without any
/// connected machine there is nothing to choose: Paseo's `connectHost`.
pub(super) fn body(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.words.history();
    let needle = shell.areas.import.search.text(cx).trim().to_lowercase();
    let hosts: Vec<&Machine> = shell
        .fleet
        .online()
        .filter(|machine| matches(machine, &needle))
        .collect();
    let note = if shell.fleet.online().next().is_none() {
        Some(words.connect_host())
    } else if hosts.is_empty() {
        Some(words.no_hosts())
    } else {
        None
    };
    div()
        .flex()
        .flex_col()
        .min_w_0()
        .children(note.map(|note| {
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground_muted)
                .child(note.to_owned())
        }))
        .children(
            hosts
                .into_iter()
                .map(|machine| row(shell, machine, tokens, cx)),
        )
}

/// Paseo's `HostChooserRow`: the status dot in an 18 slot, the name over its
/// id, and the `Server` mark; 56 high, 16 by 12, `surface1` under the pointer,
/// reaching the card's edges as the chooser's list does.
fn row(
    shell: &Shell,
    machine: &Machine,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let neutral = tokens.neutral;
    let selector = format!("history-host-{}", machine.name);
    let act = listener(shell, Action::Host(machine.name.clone()));
    let hover = neutral.surface1;
    div()
        .id(ElementId::Name(selector.clone().into()))
        .debug_selector(move || selector.clone())
        .role(gpui::Role::Button)
        .aria_label(machine.name.clone())
        .flex()
        .flex_row()
        .items_center()
        .gap(px(S3))
        .min_h(px(56.0))
        .mx(px(EDGE))
        .px(px(S4))
        .py(px(S3))
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .active(move |style| style.bg(hover))
        .child(
            div()
                .w(px(18.0))
                .flex()
                .items_center()
                .justify_center()
                .child(vessel_app_kit::dot(tokens.st_ready)),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(S1))
                .child(
                    div()
                        .type_(Type::Body)
                        .text_color(neutral.foreground)
                        .truncate()
                        .child(machine.name.clone()),
                )
                .children(machine.node.map(|node| {
                    div()
                        .type_(Type::Meta)
                        .text_color(neutral.foreground_muted)
                        .truncate()
                        .child(node.to_string())
                })),
        )
        .child(vessel_app_kit::glyph(
            Glyph::Server,
            vessel_app_kit::theme::icon::SM,
            neutral.foreground_muted,
        ))
        .on_click(cx.listener(move |shell, _, window, cx| act(shell, window, cx)))
        .into_any_element()
}

impl Shell {
    /// Registration runs immediately after the sole `Fleet` installation, before
    /// another retained input can dispatch. Session activity alone is irrelevant.
    pub(crate) fn history_fleet(&mut self, earlier: &Fleet) {
        if !self.areas.import.open {
            return;
        }
        let controls = |machine: &Machine| {
            (
                machine.name.clone(),
                machine.connection_context(),
                machine.agents.clone(),
            )
        };
        if earlier
            .online()
            .map(controls)
            .ne(self.fleet.online().map(controls))
        {
            let _current = self.areas.import.advance_visit();
        }
    }

    /// An exact retired context invalidates every control drawn for it. A
    /// delayed retirement for a context absent from the current fleet and the
    /// opening must not touch a successor opening. No node lookup occurs.
    pub(crate) fn history_retired(&mut self, context: &ConnectionTag, cx: &mut Context<'_, Self>) {
        if self.areas.import.open
            && (self
                .areas
                .import
                .model
                .opening()
                .is_some_and(|opening| &opening.context.tag() == context)
                || self.fleet.online().any(|machine| {
                    machine
                        .connection_context()
                        .is_some_and(|current| &current.tag() == context)
                }))
        {
            let _current = self.areas.import.advance_visit();
            // DeviceRetired may be irrelevant to Devices and return before the
            // outer dispatch notify; this consumer still changed its own state.
            cx.notify();
        }
    }
}
