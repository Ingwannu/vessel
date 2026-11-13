//! Projected shared names, connections and secret-name presence only.
use super::{Shell, frame};
use vessel_app_kit::ui::{
    AnyElement, FluentBuilder as _, IntoElement as _, ParentElement as _, Styled, div, px,
};
use vessel_app_kit::{Tokens, settings, theme::spacing};
use vessel_app_view::sync::{FileKind, Part, Snapshot, connection_variant};

pub(super) fn publication(
    shell: &Shell,
    snapshot: &Snapshot,
    tokens: &Tokens,
    compact: bool,
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
            Some(frame::mono(file.path.as_str().to_owned(), compact).into_any_element()),
            index == 0,
            compact,
            tokens,
        ));
    }
    let mut tools = settings::card(tokens).child(frame::message(words.tool_hint, tokens));
    if published.servers.is_empty() {
        tools = tools.child(frame::message(words.empty_tools, tokens));
    }
    for name in &published.servers {
        tools = tools.child(settings::row_at(
            words.tool,
            None,
            None,
            Some(frame::mono(name.as_str().to_owned(), compact).into_any_element()),
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
                    Some(words.connection_details(node).into()),
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
    for secret in snapshot.secret_rows() {
        card = card.child(settings::row_at(
            words.usage(secret.usage),
            None,
            None,
            Some(
                div()
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap(px(spacing::S2))
                    .min_w_0()
                    .max_w_full()
                    .when(compact, Styled::w_full)
                    .when(!compact, Styled::flex_1)
                    .child(frame::mono(secret.name.as_str().to_owned(), compact))
                    .child(frame::badge(
                        words.presence(secret.presence),
                        secret.presence.variant(),
                        tokens,
                    ))
                    .into_any_element(),
            ),
            false,
            compact,
            tokens,
        ));
    }
    settings::section(words.secrets, card, tokens).into_any_element()
}
