//! Every desktop window follows the single canonical preferences actor.

use super::Desktop;
use crate::shell::Shell;
use gpui::{Context, Window};
use vessel_app_link::desktop::preferences::{Observation, Outcome, Snapshot};
use vessel_app_link::{Command, Prefs};

pub(crate) fn start(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    let Some(desktop) = cx.try_global::<Desktop>() else {
        return;
    };
    let (client, mut updates) = desktop.preferences.client();
    let previous = shell.prefs.layouts.clone();
    let initial = updates.borrow_and_update().clone();
    shell.areas.desktop.preferences = Some(client);
    // Creation may have captured an older snapshot than this new subscription.
    apply_with_layouts(shell, &initial, &previous, window, cx);
    cx.spawn_in(window, async move |this, cx| {
        while updates.changed().await.is_ok() {
            let snapshot = updates.borrow_and_update().clone();
            if this
                .update_in(cx, |shell, window, cx| apply(shell, &snapshot, window, cx))
                .is_err()
            {
                break;
            }
        }
    })
    .detach();
}

pub(crate) fn submit(
    shell: &Shell,
    command: Command,
) -> Result<Option<Command>, vessel_app_link::Unsent> {
    let Some(client) = &shell.areas.desktop.preferences else {
        return Ok(Some(command));
    };
    match command {
        Command::SavePrefs(prefs) => match client.publish(&prefs) {
            Ok(()) => Ok(None),
            Err(why) => Err(vessel_app_link::Unsent::new(why, Command::SavePrefs(prefs))),
        },
        forward @ (Command::RetireConnection(_)
        | Command::LocalNode(_)
        | Command::Onboarding(_)
        | Command::History(_)
        | Command::Agents { .. }
        | Command::Machines(_)
        | Command::Reload
        | Command::Devices { .. }
        | Command::NodeSettings(_)
        | Command::Sync { .. }
        | Command::Open { .. }
        | Command::Prompt { .. }
        | Command::Start { .. }
        | Command::Options { .. }
        | Command::SaveDrafts(_)
        | Command::Files(_)
        | Command::KeepCommands { .. }
        | Command::Resume { .. }
        | Command::Stop { .. }
        | Command::Answer { .. }
        | Command::Act { .. }
        | Command::Pair { .. }
        | Command::Workspace { .. }
        | Command::Search(_)) => Ok(Some(forward)),
    }
}

pub(crate) fn refused(shell: &mut Shell, unsent: &vessel_app_link::Unsent) {
    if let Command::SavePrefs(prefs) = unsent.command.as_ref() {
        shell.areas.desktop.rollback = Some(prefs.as_ref().clone());
    }
}

pub(crate) fn rollback(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    if let Some(expected) = shell.areas.desktop.rollback.take()
        && shell.prefs == expected
        && let Some(desktop) = cx.try_global::<Desktop>()
    {
        let snapshot = desktop.preferences.snapshot();
        apply(shell, &snapshot, window, cx);
    }
}

fn apply(shell: &mut Shell, snapshot: &Snapshot, window: &mut Window, cx: &mut Context<'_, Shell>) {
    let Some(previous) = shell
        .areas
        .desktop
        .preferences
        .as_ref()
        .map(vessel_app_link::desktop::preferences::Client::canonical_preferences)
    else {
        return;
    };
    apply_with_layouts(shell, snapshot, &previous.layouts, window, cx);
}

fn apply_with_layouts(
    shell: &mut Shell,
    snapshot: &Snapshot,
    previous: &[vessel_app_view::layout::RememberedLayout],
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    let Some(client) = &shell.areas.desktop.preferences else {
        return;
    };
    let next = match client.observe(snapshot) {
        Observation::Stale => return,
        Observation::Applied(prefs) => prefs,
        Observation::Refused { prefs, failure } => {
            tracing::warn!(%failure,"pending preferences exceed their model bound");
            shell.notice = Some(shell.words.desktop_shell().preferences_failed().to_owned());
            prefs
        }
    };
    let canonical = snapshot.preferences();
    if previous != canonical.layouts.as_slice() {
        shell.layout_preferences_applied(previous, &canonical.layouts, cx);
    }
    if shell.prefs != *next {
        apply_preferences(shell, (*next).clone(), window, cx);
    }
    if let Outcome::Failed(error) = snapshot.outcome() {
        tracing::warn!(%error,"preferences change was refused");
        shell.notice = Some(shell.words.desktop_shell().preferences_failed().to_owned());
    }
    cx.notify();
}

fn apply_preferences(
    shell: &mut Shell,
    next: Prefs,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    let old = std::mem::replace(&mut shell.prefs, next);
    if old.settings.language != shell.prefs.settings.language {
        match shell.language.prepare(shell.prefs.settings.language) {
            Ok(words) => shell.apply_language(words, window, cx),
            Err(error) => {
                tracing::error!(%error,"shared preferences language could not be prepared");
            }
        }
    }
    crate::areas::settings::apply_theme(shell, window, cx);
    crate::areas::settings::apply_composer(shell, window, cx);
    crate::areas::settings::apply_text_sizes(shell, cx);
    crate::areas::session::set_content_width(shell.prefs.settings.appearance.content_width);
    shell.conversation_display(shell.prefs.settings.appearance.conversation);
    crate::motion::animations(shell.prefs.settings.appearance.animations.on(), cx);
    if old.manual_updates != shell.prefs.manual_updates {
        shell.sync_update_automatically();
    }
    window.refresh();
}

#[cfg(test)]
mod startup_tests;
