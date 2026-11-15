//! Local policy and controller operations reuse the shared settings controls.

use super::LocalTarget;
use super::{Action, AfterQuit, Context, Intent, Logs, Management, Prompt, Shell, Status, Window};
use vessel_app_kit::ui::{
    AnyElement, FluentBuilder as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div,
};
use vessel_app_kit::{Tokens, outline_button, settings};

/// Host-context navigation does not authorize a process operation.
pub(crate) fn overview(
    shell: &Shell,
    target: &LocalTarget,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.local_node();
    let provenance = match target.provenance {
        vessel_app_view::local_node::LocalProvenance::OwnedChild => words.owned(),
        vessel_app_view::local_node::LocalProvenance::AttachedManagedHome => words.attached(),
    };
    let navigation = outline_button("local-node-general", words.manage(), true, tokens)
        .on_click(cx.listener(|shell, _event, _window, cx| {
            shell.settings_section(vessel_app_view::settings::Section::General, cx);
        }))
        .into_any_element();
    settings::section(
        words.title(),
        settings::card(tokens).child(settings::row_at(
            words.status(),
            Some(provenance.into()),
            None,
            Some(navigation),
            false,
            compact,
            tokens,
        )),
        tokens,
    )
    .into_any_element()
}

pub(crate) fn section(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.local_node();
    let status = shell.areas.local_node.snapshot.as_ref().map_or_else(
        || crate::local_node::unavailable_status(cx),
        |snapshot| snapshot.status,
    );
    let working = status == Status::Working
        || shell.areas.local_node.action_pending
        || crate::quit::is_finalizing(cx);
    let enabled = shell.prefs.local_node.management == Management::Enabled;
    let keep = shell.prefs.local_node.after_quit == AfterQuit::KeepRunning;
    let manage = settings::switch(
        "manage-local-node",
        enabled,
        status != Status::NotInstalled && !working,
        working,
        0,
        tokens,
    )
    .on_click(cx.listener(move |shell, _event, _window, cx| shell.manage_local_node(!enabled, cx)))
    .into_any_element();
    let after = settings::switch("keep-local-node", keep, !working, working, 0, tokens)
        .on_click(cx.listener(move |shell, _event, _window, cx| shell.keep_local_node(!keep, cx)))
        .into_any_element();
    let failed = shell
        .areas
        .local_node
        .snapshot
        .as_ref()
        .is_some_and(|snapshot| snapshot.failure.is_some());
    let protected = shell
        .areas
        .local_node
        .snapshot
        .as_ref()
        .is_some_and(|snapshot| snapshot.active_sessions().is_some());
    let controls = controls(shell, status, tokens, cx);
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
            Some(status_text(shell, status).into()),
            None,
            Some(controls),
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
                protected.then(|| words.protected_note().into()),
                None,
                false,
                tokens,
            ))
        });
    settings::section(words.title(), card, tokens).into_any_element()
}

fn controls(
    shell: &Shell,
    status: Status,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.local_node();
    let idle = status != Status::Working
        && !shell.areas.local_node.action_pending
        && !crate::quit::is_finalizing(cx);
    let readable = cx
        .try_global::<crate::local_node::Managed>()
        .is_some_and(|managed| managed.controller.is_some());
    let mut controls = div().flex().flex_row().flex_wrap();
    if matches!(
        status,
        Status::Paused | Status::Stopped | Status::Disconnected | Status::Failed | Status::Kept(_)
    ) {
        controls = controls.child(
            outline_button(
                "local-node-start",
                if matches!(status, Status::Kept(_)) {
                    words.reconnect()
                } else {
                    words.start()
                },
                readable && idle,
                tokens,
            )
            .on_click(cx.listener(|shell, _event, _window, cx| {
                shell.local_node_action(Action::Start, cx);
            })),
        );
    }
    if let Some(lease) = shell
        .areas
        .local_node
        .snapshot
        .as_ref()
        .and_then(vessel_app_link::local_node::Snapshot::control)
    {
        let restart = lease.clone();
        controls = controls
            .child(
                outline_button("local-node-restart", words.restart(), idle, tokens).on_click(
                    cx.listener(move |shell, _event, _window, cx| {
                        shell.local_node_action(Action::Restart(restart.clone()), cx);
                    }),
                ),
            )
            .child(
                outline_button("local-node-stop", words.stop(), idle, tokens).on_click(
                    cx.listener(move |shell, _event, _window, cx| {
                        shell.local_node_action(Action::Stop(lease.clone()), cx);
                    }),
                ),
            );
    }
    controls
        .child(
            outline_button("local-node-logs", words.logs(), readable && idle, tokens).on_click(
                cx.listener(|shell, _event, window, cx| shell.read_local_logs(window, cx)),
            ),
        )
        .child(
            outline_button("local-log-path", words.copy_log_path(), readable, tokens).on_click(
                cx.listener(|_shell, _event, _window, cx| Shell::copy_local_log_path(cx)),
            ),
        )
        .into_any_element()
}

fn prompt_content(
    shell: &Shell,
    prompt: &Prompt,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> vessel_app_kit::dialog::DialogContent {
    use vessel_app_kit::dialog::{DialogContent, DialogSize};
    let words = shell.words.local_node();
    let title = match prompt.intent {
        Intent::Quit => words.quit_refused(),
        Intent::Stop => words.confirm_stop(),
        Intent::Restart => words.confirm_restart(),
        Intent::Update => words.confirm_update(),
    };
    let body = div()
        .flex()
        .flex_col()
        .child(words.protected_note())
        .children(
            prompt
                .snapshot
                .active_sessions()
                .into_iter()
                .flatten()
                .map(|id| {
                    let name = shell
                        .fleet
                        .machines
                        .iter()
                        .filter(|machine| matches!(prompt.snapshot.status, Status::Owned(node) if machine.node == Some(node)))
                        .flat_map(|machine| &machine.sessions)
                        .find(|card| card.session == *id)
                        .map(|card| card.title.clone());
                    div().child(
                        name.filter(|name| !name.is_empty())
                            .unwrap_or_else(|| id.to_string()),
                    )
                }),
        );
    let footer = prompt_footer(shell, prompt, tokens, cx);
    DialogContent {
        title: title.into(),
        body: body.into_any_element(),
        footer,
        size: DialogSize::Normal,
    }
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

pub(crate) fn dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    use vessel_app_kit::dialog::{DialogContent, DialogSize, dialog};
    let words = shell.words.local_node();
    if let Some(prompt) = &shell.areas.local_node.prompt {
        let content = prompt_content(shell, prompt, tokens, cx);
        let identity = std::rc::Rc::clone(&prompt.identity);
        return Some(dialog(
            &shell.areas.local_node.prompt_dialog,
            content,
            tokens,
            window,
            crate::areas::draft::compact(window),
            move |shell: &mut Shell, window, cx| {
                if shell.dismiss_local_prompt(&identity, window, cx) {
                    cx.notify();
                }
            },
            cx,
        ));
    }
    let body = match &shell.areas.local_node.logs {
        Logs::Closed => return None,
        Logs::Loading(_) => div().child(words.working()),
        Logs::Ready(text) => div().child(text.as_str().to_owned()),
        Logs::Failed => div().child(words.logs_unavailable()),
    };
    let footer = vec![
        outline_button("local-logs-refresh", words.refresh_logs(), true, tokens)
            .on_click(cx.listener(|shell, _event, window, cx| shell.read_local_logs(window, cx)))
            .into_any_element(),
    ];
    Some(dialog(
        &shell.areas.local_node.logs_dialog,
        DialogContent {
            title: words.logs().into(),
            body: body.into_any_element(),
            footer,
            size: DialogSize::Wide,
        },
        tokens,
        window,
        crate::areas::draft::compact(window),
        |shell: &mut Shell, window, cx| {
            shell.areas.local_node.logs_dialog.dismissed(window, cx);
            shell.areas.local_node.logs = Logs::Closed;
            cx.notify();
        },
        cx,
    ))
}

fn prompt_footer(
    shell: &Shell,
    prompt: &Prompt,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Vec<AnyElement> {
    let words = shell.words.local_node();
    let mut footer = Vec::new();
    if matches!(prompt.intent, Intent::Quit) {
        let identity = std::rc::Rc::clone(&prompt.identity);
        footer.push(
            outline_button("local-quit-keep", words.keep_quit(), true, tokens)
                .on_click(cx.listener(move |shell, _event, window, cx| {
                    if !shell.dismiss_local_prompt(&identity, window, cx) {
                        return;
                    }
                    crate::local_node::request_keep_quit(cx);
                }))
                .into_any_element(),
        );
    }
    footer.extend(interrupt_button(shell, prompt, tokens, cx));
    let identity = std::rc::Rc::clone(&prompt.identity);
    footer.push(
        outline_button("local-confirm-cancel", words.cancel(), true, tokens)
            .on_click(cx.listener(move |shell, _event, window, cx| {
                if shell.dismiss_local_prompt(&identity, window, cx) {
                    cx.notify();
                }
            }))
            .into_any_element(),
    );
    footer
}

fn interrupt_button(
    shell: &Shell,
    prompt: &Prompt,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let words = shell.words.local_node();
    let confirmation = prompt.snapshot.confirmation()?;
    let identity = std::rc::Rc::clone(&prompt.identity);
    let intent = prompt.intent;
    let label = match intent {
        Intent::Quit => words.interrupt_quit(),
        Intent::Stop => words.interrupt_stop(),
        Intent::Restart => words.interrupt_restart(),
        Intent::Update => words.interrupt_update(),
    };
    Some(
        outline_button("local-confirm-interrupt", label, true, tokens)
            .on_click(cx.listener(move |shell, _event, window, cx| {
                if !shell.dismiss_local_prompt(&identity, window, cx) {
                    return;
                }
                match intent {
                    Intent::Quit => {
                        crate::local_node::request_confirmed_quit(confirmation.clone(), cx);
                    }
                    Intent::Stop => shell
                        .local_node_action(Action::InterruptConfirmed(confirmation.clone()), cx),
                    Intent::Update => {
                        shell.restart_to_update_confirmed(confirmation.clone(), cx);
                    }
                    Intent::Restart => shell.local_node_action(
                        Action::RestartInterruptConfirmed(confirmation.clone()),
                        cx,
                    ),
                }
            }))
            .into_any_element(),
    )
}
