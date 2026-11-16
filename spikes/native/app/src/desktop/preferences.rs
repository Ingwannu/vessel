//! Every desktop window follows the single canonical preferences actor.

use super::Desktop;
use crate::shell::Shell;
use gpui::{Context, Window};
use std::rc::Rc;
use vessel_app_link::desktop::preferences::Receipt;
use vessel_app_link::desktop::preferences::{Observation, Outcome, Snapshot};
use vessel_app_link::{Command, Prefs};

const RECENT_FAILURE_CAPACITY: usize = 64;

mod scrollback;
pub(crate) use scrollback::drain_terminal;

#[cfg(test)]
mod tests;

/// One synchronous model edit captures only its own existing SavePrefs attempt.
#[derive(Default)]
pub(crate) struct ReceiptCapture {
    cycle: Rc<()>,
    latest: Option<Result<Receipt, vessel_app_link::Rejected>>,
    reported_failures: std::collections::VecDeque<vessel_app_link::desktop::preferences::Revision>,
}

/// Reuses Settings' validator/live application; never publishes a second command.
/// None means the model refused the edit or this shell has no native writer.
pub(crate) fn prefer_with_receipt(
    shell: &mut Shell,
    change: &vessel_app_view::prefs::PreferenceChange,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<Result<Receipt, vessel_app_link::Rejected>> {
    let cycle = Rc::clone(&shell.areas.desktop.receipts.borrow().cycle);
    shell.prefer(change, window, cx);
    let mut capture = shell.areas.desktop.receipts.borrow_mut();
    if Rc::ptr_eq(&cycle, &capture.cycle) {
        None
    } else {
        capture.latest.take()
    }
}

/// The caller admits its current UI attempt first. Exact failures are still
/// reported when watch delivery coalesced them behind a newer success, without
/// applying the older snapshot or showing the same failure twice.
pub(crate) fn acknowledge(
    shell: &mut Shell,
    snapshot: &Snapshot,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    apply(shell, snapshot, window, cx);
    report_failure(shell, snapshot);
    cx.notify();
}

pub(crate) fn start(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    let Some(desktop) = cx.try_global::<Desktop>() else {
        return;
    };
    let (client, mut updates) = desktop.preferences.client();
    let previous = shell.prefs.layouts.clone();
    let initial = updates.borrow_and_update().clone();
    shell.areas.desktop.preferences = Some(client);
    // A writer can commit between window construction and subscription. Its
    // initial watch value may never trigger changed(), and the retained layout
    // cache still belongs to the actual window snapshot, not the new Client.
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
        Command::SavePrefs(prefs) => {
            let receipt = client.publish_with_receipt(&prefs);
            let refused = receipt.as_ref().err().copied();
            let mut capture = shell.areas.desktop.receipts.borrow_mut();
            capture.cycle = Rc::new(());
            capture.latest = Some(receipt);
            match refused {
                None => Ok(None),
                Some(why) => Err(vessel_app_link::Unsent::new(why, Command::SavePrefs(prefs))),
            }
        }
        forward @ (Command::History(_)
        | Command::RetireConnection(_)
        | Command::Agents { .. }
        | Command::SessionAccess(_)
        | Command::Sessions(_)
        | Command::SessionLifecycle(_)
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
    // Layout owns retained dirty/ephemeral trees. Rebase its cache against
    // canonical snapshots before installing this window's optimistic projection.
    if previous != canonical.layouts.as_slice() {
        shell.layout_preferences_applied(previous, &canonical.layouts, cx);
    }
    if shell.prefs != *next {
        apply_preferences(shell, (*next).clone(), window, cx);
    }
    // A failed watch snapshot can follow coalesced successful terminal writes.
    // Drain the writer's Saved aggregate, never this optimistic projection.
    let _delivery = drain_terminal(shell, cx);
    report_failure(shell, snapshot);
    cx.notify();
}

fn report_failure(shell: &mut Shell, snapshot: &Snapshot) {
    let Outcome::Failed(error) = snapshot.outcome() else {
        return;
    };
    let mut capture = shell.areas.desktop.receipts.borrow_mut();
    if capture.reported_failures.contains(&snapshot.revision()) {
        return;
    }
    if capture.reported_failures.len() >= RECENT_FAILURE_CAPACITY {
        let _oldest = capture.reported_failures.pop_front();
    }
    capture.reported_failures.push_back(snapshot.revision());
    drop(capture);
    tracing::warn!(%error,"preferences change was refused");
    shell.notice = Some(shell.words.desktop_shell().preferences_failed().to_owned());
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
