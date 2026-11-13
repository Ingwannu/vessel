//! Projected shared names, connections and secret-name presence only.
use super::{Shell, frame};
use vessel_app_kit::ui::{
    AnyElement, Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    ParentElement as _, Stateful, Styled, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, fonts, settings, theme::spacing};
use vessel_app_view::sync::{FileKind, Part, Snapshot, connection_variant};

pub(super) fn publication(
    shell: &Shell,
    snapshot: &Snapshot,
    tokens: &Tokens,
    compact: bool,
    window: &Window,
) -> AnyElement {
    let Ok(published) = &snapshot.published else {
        return div().into_any_element();
    };
    let words = shell.t.lang.sync();
    let mut files = settings::card(tokens);
    if published.files.is_empty() {
        files = files.child(frame::message(words.empty_files, tokens));
    }
    for (index, file) in published.files.iter().enumerate() {
        files = files.child(settings::row_at(
            words.file_kind(FileKind::of(&file.path)),
            Some(words.file_metadata(file.size, file.mode).into()),
            None,
            Some(path(shell, file.path.as_str(), tokens, compact, window).into_any_element()),
            index == 0,
            compact,
            tokens,
        ));
    }
    let mut tools = settings::card(tokens).child(frame::message(words.tool_hint, tokens));
    if published.servers.is_empty() {
        tools = tools.child(frame::message(words.empty_tools, tokens));
    }
    // The section says what these are; each row is the server's name (the
    // same title on every row said nothing).
    for name in &published.servers {
        tools = tools.child(settings::row_with_content(
            frame::mono(name.as_str().to_owned(), compact).into_any_element(),
            None,
            false,
            compact,
            tokens,
        ));
    }
    div()
        .child(settings::section(words.files, files, tokens))
        .child(settings::section(words.tools, tools, tokens))
        .into_any_element()
}

pub(super) fn connections(
    shell: &Shell,
    snapshot: &Snapshot,
    tokens: &Tokens,
    compact: bool,
) -> AnyElement {
    let words = shell.t.lang.sync();
    let mut card = settings::card(tokens).child(frame::message(words.connections_hint, tokens));
    match &snapshot.connections {
        Err(why) => card = card.child(frame::problem(shell, Part::Connections, *why, tokens)),
        Ok(nodes) => {
            if nodes.is_empty() {
                card = card.child(frame::message(words.empty_connections, tokens));
            }
            for node in nodes {
                card = card.child(settings::row_at(
                    node.name.clone(),
                    Some(words.connection_version(node.protocol).into()),
                    None,
                    Some(
                        frame::badge(
                            words.connection_state(node.reachable),
                            connection_variant(node),
                            tokens,
                        )
                        .into_any_element(),
                    ),
                    false,
                    compact,
                    tokens,
                ));
            }
        }
    }
    settings::section(words.connections, card, tokens).into_any_element()
}

pub(super) fn secrets(
    shell: &Shell,
    snapshot: &Snapshot,
    tokens: &Tokens,
    compact: bool,
) -> AnyElement {
    let words = shell.t.lang.sync();
    let mut card = settings::card(tokens).child(frame::message(words.secret_hint, tokens));
    if let Err(why) = snapshot.secrets {
        card = card.child(frame::problem(shell, Part::Secrets, why, tokens));
    }
    if let Some(empty) = snapshot.secret_empty() {
        card = card.child(frame::message(words.secret_empty(empty), tokens));
    }
    // The name is the row's title (in the code face), what uses it the
    // hint, whether this machine holds it the badge at the row's end, as
    // every other row with a state.
    for secret in snapshot.secret_rows() {
        card = card.child(settings::row_with_content(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(spacing::S1))
                .child(frame::mono(secret.name.as_str().to_owned(), compact))
                .child(
                    div()
                        .type_(Type::Meta)
                        .text_color(tokens.neutral.foreground_muted)
                        .child(words.usage(secret.usage)),
                )
                .into_any_element(),
            Some(
                frame::badge(
                    words.presence(secret.presence),
                    secret.presence.variant(),
                    tokens,
                )
                .into_any_element(),
            ),
            false,
            compact,
            tokens,
        ));
    }
    settings::section(words.secrets, card, tokens).into_any_element()
}

/// A shared file's path on one line, cut in the middle when it is longer
/// than its room (kit `fonts::elide_middle`), the whole path in a tooltip:
/// the brief's rule for a path (it wrapped at its slashes and hyphens over
/// three lines).
fn path(
    shell: &Shell,
    text: &str,
    tokens: &Tokens,
    compact: bool,
    window: &Window,
) -> Stateful<Div> {
    // A phone's row content width, under the row's words; beside them on a
    // pointer's width.
    let width = if compact {
        settings::PHONE_ROW_INSET.mul_add(-1.0, f32::from(window.viewport_size().width))
    } else {
        PATH_WIDTH
    };
    let shown = fonts::elide_middle(window, text, Type::Mono, width);
    let cut = shown != text;
    let element = div()
        .id(gpui::SharedString::from(format!("sync-path-{text}")))
        .w(px(width))
        .flex_none()
        .flex()
        .when(!compact, Styled::justify_end)
        .type_(Type::Mono)
        .whitespace_nowrap()
        .text_color(tokens.neutral.foreground)
        .child(shown);
    if !cut {
        return element;
    }
    let state = &shell.areas.settings.sync;
    let place = std::rc::Rc::clone(state.paths.borrow_mut().entry(text.to_owned()).or_default());
    state.tips.on(element, &place, text.to_owned(), compact)
}

/// A path's room beside its row's words on a pointer's width.
const PATH_WIDTH: f32 = 320.0;
