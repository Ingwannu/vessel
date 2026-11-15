//! Git joins the sole layout's synchronous retirement boundary once at startup.
//!
//! [Decision Log]
//! - Reuse the reviewed layout hook, empty owner table and duplicate refusal.
//!   Target equality alone cannot detect a tab going away and coming back.
//! - Drop only matching pane UI/read tickets after a committed mutation. The
//!   Link owns detached calls; global sent/pending write receipts stay intact.
//! - Startup failure keeps pane reads unavailable and retains its typed cause.
//!   No RPC, layout reentry, second owner map or pending-task abort is allowed.
//! - C5's drawn-source invalidation helper remains a separate unready join;
//!   the legacy write barrier stays closed until that producer is consumed.
use crate::areas::layout::registry::retirement::{Notice, Owner};
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_view::layout::LayoutError;
use vessel_app_view::workspace::git::panel::ReadFailure;

/// Runtime registration only; root and connection authority remain separate.
#[derive(Default)]
pub(super) enum Registration {
    #[default]
    Uninitialized,
    Ready,
    Refused(LayoutError),
}
impl Registration {
    /// Missing registration is a settled local failure, never a loading read.
    pub(super) fn problem(&self) -> Option<ReadFailure> {
        match self {
            Self::Uninitialized => Some(ReadFailure::OwnerUnavailable { cause: None }),
            Self::Ready => None,
            Self::Refused(why) => Some(ReadFailure::OwnerUnavailable { cause: Some(*why) }),
        }
    }
}

/// Called once after Areas construction, before route arrival. A failed
/// registration never overwrites another callback or makes old data current.
pub(crate) fn start(shell: &mut Shell, cx: &mut Context<'_, Shell>) -> Result<(), LayoutError> {
    let result = shell.layout_register_retirement(Owner::Git, retired);
    shell.areas.git.panels.initialize(result);
    cx.notify();
    result
}

fn retired(shell: &mut Shell, notice: &Notice, cx: &mut Context<'_, Shell>) {
    if shell.areas.git.panels.retire_owner(notice) {
        tracing::debug!(reason = ?notice.reason(), "retired the original Git pane view");
        cx.notify();
    }
}
