//! Local policy and controller operations reuse the shared settings controls.

use super::*;
use vessel_app_kit::ui::{
    AnyElement, FluentBuilder as _, InteractiveElement as _, IntoElement as _, ParentElement as _,
    Styled as _, div,
};
use vessel_app_kit::{Tokens, outline_button, settings};

pub(crate) fn section(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.local_node();
    let status = shell
        .areas
        .local_node
        .snapshot
        .as_ref()
        .map_or(Status::Failed, |snapshot| snapshot.status);
    let working = status == Status::Working;
    let installed = status != Status::NotInstalled;
    let enabled = shell.prefs.local_node.management == Management::Enabled;
    let keep = shell.prefs.local_node.after_quit == AfterQuit::KeepRunning;
    let manage = settings::switch(
        "manage-local-node",
        enabled,
        installed && !working,
        working,
        0,
        tokens,
    )
    .on_click(cx.listener(move |shell, _event, _window, cx| shell.manage_local_node(!enabled, cx)))
    .into_any_element();
    let after = settings::switch("keep-local-node", keep, true, false, 0, tokens)
        .on_click(cx.listener(move |shell, _event, _window, cx| shell.keep_local_node(!keep, cx)))
        .into_any_element();
    let mut controls = div().flex().flex_row().flex_wrap();
    if matches!(
        status,
        Status::Paused | Status::Stopped | Status::Disconnected | Status::Failed
    ) && installed
    {
        controls = controls.child(
            outline_button("local-node-start", words.start(), !working, tokens).on_click(
                cx.listener(|shell, _event, _window, cx| {
                    shell.local_node_action(Action::Start, cx)
                }),
            ),
        );
    }
    if matches!(status, Status::Owned(_)) {
        controls = controls
            .child(
                outline_button("local-node-restart", words.restart(), !working, tokens).on_click(
                    cx.listener(|shell, _event, _window, cx| {
                        shell.local_node_action(Action::Restart, cx)
                    }),
                ),
            )
            .child(
                outline_button("local-node-stop", words.stop(), !working, tokens).on_click(
                    cx.listener(|shell, _event, _window, cx| {
                        shell.local_node_action(Action::Stop, cx)
                    }),
                ),
            );
    }
    let note = status_text(shell, status);
    let failed = shell
        .areas
        .local_node
        .snapshot
        .as_ref()
        .is_some_and(|snapshot| snapshot.failure.is_some());
    let card = settings::card(tokens)
        .child(settings::row_at(
            words.manage(),
            Some(words.manage_note().into()),
            None,
            Some(manage),
            true,
            compact,
            tokens,
        ))
        .child(settings::row_at(
            words.status(),
            Some(note.into()),
            None,
            Some(controls.into_any_element()),
            false,
            compact,
            tokens,
        ))
        .child(settings::row_at(
            words.keep(),
            Some(words.keep_note().into()),
            None,
            Some(after),
            false,
            compact,
            tokens,
        ))
        .when(failed, |card| {
            card.child(settings::row(
                words.failed(),
                Some(words.protected_note().into()),
                None,
                false,
                tokens,
            ))
        })
        .when(shell.areas.local_node.quit_refused, |card| {
            let actions = div()
                .flex()
                .flex_row()
                .flex_wrap()
                .child(
                    outline_button("local-quit-keep", words.keep_quit(), true, tokens).on_click(
                        cx.listener(|shell, _event, _window, cx| {
                            shell.keep_local_node(true, cx);
                            crate::local_node::request_keep_quit(cx);
                        }),
                    ),
                )
                .child(
                    outline_button("local-quit-interrupt", words.interrupt_quit(), true, tokens)
                        .on_click(cx.listener(|_shell, _event, _window, cx| {
                            crate::local_node::request_confirmed_quit(cx)
                        })),
                )
                .child(
                    outline_button("local-quit-cancel", words.cancel_quit(), true, tokens)
                        .on_click(cx.listener(|shell, _event, _window, cx| {
                            shell.areas.local_node.quit_refused = false;
                            cx.notify();
                        })),
                );
            card.child(settings::row(
                words.quit_refused(),
                Some(words.protected_note().into()),
                Some(actions.into_any_element()),
                false,
                tokens,
            ))
        });
    settings::section(words.title(), card, tokens).into_any_element()
}

fn status_text(shell: &Shell, status: Status) -> &str {
    let words = shell.words.local_node();
    match status {
        Status::NotInstalled => words.not_installed(),
        Status::Paused => words.paused(),
        Status::Working => words.working(),
        Status::Owned(_) => words.owned(),
        Status::Attached(_) => words.attached(),
        Status::Kept(_) => words.kept(),
        Status::Stopped => words.stopped(),
        Status::Disconnected => words.disconnected(),
        Status::Failed => words.failed(),
    }
}
