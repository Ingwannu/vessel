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
struct Documents(DocumentGuard);
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
    cx.set_global(Documents(guard));
}

/// No writes/discards occur here. The final call must have no subsequent await.
pub(crate) fn admit(cx: &mut App) -> bool {
    let Some(guard) = cx.try_global::<Documents>().map(|documents| documents.0) else {
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
        let words = shell.words.local_node();
        shell.notice = Some(
            match blocked {
                DocumentReadiness::Ready => return,
                DocumentReadiness::Dirty => words.documents_dirty(),
                DocumentReadiness::Pending => words.documents_pending(),
                DocumentReadiness::Failed => words.documents_failed(),
            }
            .to_owned(),
        );
        cx.notify();
    });
    false
}
