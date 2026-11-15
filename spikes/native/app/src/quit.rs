//! One read-only document admission hook joins the existing node/prefs quit path.
//!
//! [Decision Log]
//! - Purpose: stopping a node must not turn a pending editor write into lost work.
//! - Constraint: C5 alone knows retained buffers and original write identities.
//! - Alternatives: implicit Save All, a second quit manager, another window registry.
//! - Choice: one C5 guard over the desktop's existing live Shell owners, checked
//!   before finalization and immediately before actual quit/update restart.
//! - Cost: refusal asks the person to save/retry; forced OS exit needs a separate
//!   editor-owned recovery journal and is not protected by this synchronous hook.

use crate::shell::Shell;
use gpui::{App, Global};
pub(crate) use vessel_app_view::desktop::quit::DocumentReadiness;

type DocumentGuard = fn(&Shell) -> DocumentReadiness;
struct Documents {
    guard: DocumentGuard,
    phase: Phase,
}
#[derive(Clone, Copy)]
enum Phase {
    Idle,
    Finalizing,
}
impl Global for Documents {}

/// Initial no-document policy is replaced when C5 constructs its first panel.
/// It never overwrites an already registered owner, including a prior window.
pub(crate) fn initialize(cx: &mut App) {
    if !cx.has_global::<Documents>() {
        set_document_guard(|_shell| DocumentReadiness::Ready, cx);
    }
}

/// C5 registers once from CodePanel::new before any buffer can become dirty.
/// The callback observes all active/retained buffers and original write tickets.
pub(crate) fn set_document_guard(guard: DocumentGuard, cx: &mut App) {
    if cx.has_global::<Documents>() {
        cx.global_mut::<Documents>().guard = guard;
    } else {
        cx.set_global(Documents {
            guard,
            phase: Phase::Idle,
        });
    }
}

/// Editor mutating handlers and editability consult the same app-wide phase.
pub(crate) fn is_finalizing(cx: &App) -> bool {
    cx.try_global::<Documents>()
        .is_some_and(|documents| matches!(documents.phase, Phase::Finalizing))
}

/// Coalesce requests without borrowing any Shell; admission is checked asynchronously.
pub(crate) fn begin(cx: &mut App) -> bool {
    if is_finalizing(cx) {
        return false;
    }
    initialize(cx);
    cx.global_mut::<Documents>().phase = Phase::Finalizing;
    cx.refresh_windows();
    true
}

/// A refused/failed quit restores all window editing through the same phase.
pub(crate) fn finish(cx: &mut App) {
    if is_finalizing(cx) {
        cx.global_mut::<Documents>().phase = Phase::Idle;
        cx.refresh_windows();
    }
}

/// No writes/discards occur here. The final call must have no subsequent await.
pub(crate) fn admit(cx: &mut App) -> bool {
    let Some(guard) = cx
        .try_global::<Documents>()
        .map(|documents| documents.guard)
    else {
        return true;
    };
    let blocked = crate::desktop::shells(cx)
        .into_iter()
        .filter_map(|weak| weak.upgrade())
        .map(|shell| guard(shell.read(cx)))
        .find(|readiness| *readiness != DocumentReadiness::Ready);
    let Some(blocked) = blocked else {
        return true;
    };
    crate::desktop::with_focused_shell(cx, |shell, _window, cx| {
        if admit_shell(shell, cx) {
            let _refused = report(shell, blocked, cx);
        }
    });
    false
}

/// The same owner guards additional-window/macOS close before removal.
pub(crate) fn admit_shell(shell: &mut Shell, cx: &mut gpui::Context<'_, Shell>) -> bool {
    let readiness = cx
        .try_global::<Documents>()
        .map_or(DocumentReadiness::Ready, |documents| {
            (documents.guard)(shell)
        });
    report(shell, readiness, cx)
}

fn report(
    shell: &mut Shell,
    readiness: DocumentReadiness,
    cx: &mut gpui::Context<'_, Shell>,
) -> bool {
    let words = shell.words.local_node();
    let message = match readiness {
        DocumentReadiness::Ready => return true,
        DocumentReadiness::Dirty => words.documents_dirty(),
        DocumentReadiness::Pending => words.documents_pending(),
        DocumentReadiness::Failed => words.documents_failed(),
    };
    shell.notice = Some(message.to_owned());
    cx.notify();
    false
}

#[cfg(test)]
mod tests;
