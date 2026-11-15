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
type NodeDocumentGuard = fn(&Shell, vessel_proto::NodeId) -> DocumentReadiness;
struct Documents {
    guard: DocumentGuard,
    node_guard: Option<NodeDocumentGuard>,
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

/// C5 registers once from `CodePanel::new` before any buffer can become dirty.
/// The callback observes all active/retained buffers and original write tickets.
pub(crate) fn set_document_guard(guard: DocumentGuard, cx: &mut App) {
    if cx.has_global::<Documents>() {
        cx.global_mut::<Documents>().guard = guard;
    } else {
        cx.set_global(Documents {
            guard,
            node_guard: None,
            phase: Phase::Idle,
        });
    }
}

/// C5 supplies a facts filter over the same retained owners and original writes.
/// The node ID grants no process authority; the controller validates its lease.
/// Until registered, the global guard conservatively protects every node.
// C5 promotes this setter with its actual retained-buffer owner registration.
#[cfg(test)]
pub(crate) fn set_node_document_guard(guard: NodeDocumentGuard, cx: &mut App) {
    initialize(cx);
    cx.global_mut::<Documents>().node_guard = Some(guard);
}

#[must_use]
pub(crate) fn readiness_on(
    shell: &Shell,
    node: vessel_proto::NodeId,
    cx: &App,
) -> DocumentReadiness {
    match cx
        .try_global::<Documents>()
        .and_then(|documents| documents.node_guard)
    {
        Some(guard) => guard(shell, node),
        None => readiness(shell, cx),
    }
}

/// Editor mutating handlers and editability consult the same app-wide phase.
#[must_use]
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

/// One registered owner reports facts; callers cannot manufacture save success.
#[must_use]
pub(crate) fn readiness(shell: &Shell, cx: &App) -> DocumentReadiness {
    cx.try_global::<Documents>()
        .map_or(DocumentReadiness::Ready, |documents| {
            (documents.guard)(shell)
        })
}

/// The same owner guards additional-window/macOS close before removal.
#[must_use]
pub(crate) fn admit_shell(shell: &mut Shell, cx: &mut gpui::Context<'_, Shell>) -> bool {
    if is_finalizing(cx) {
        // The frame projects temporary finalization status without replacing
        // an accepted editor error or another area's retained notice.
        cx.notify();
        return false;
    }
    let readiness = readiness(shell, cx);
    report(shell, readiness, cx)
}

/// Render a shared policy refusal without changing any document.
pub(crate) fn report(
    shell: &mut Shell,
    readiness: DocumentReadiness,
    cx: &mut gpui::Context<'_, Shell>,
) -> bool {
    let words = shell.words.quit();
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
