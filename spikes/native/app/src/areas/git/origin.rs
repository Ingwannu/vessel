//! Unstamped legacy requests are intentionally unavailable, not alias-routed.
//!
//! [Decision Log]
//! - Existing asset: C5 owns the one shown-Changes controller; G4 owns original
//!   connection admission. Its checked stamped acquisition/Explorer join is pending.
//! - A legacy GitWrite carries no original authenticated read receipt. Looking up
//!   the current Machine, or comparing paths/counts, cannot supply that authority.
//! - Choice: retire this unscoped sending path and use the shared Info guidance.
//!   Future writes consume C5's actual opaque receipt and G4's selected typed call;
//!   do not re-enable this alias-only transport when that producer lands.
//! - Effect: no request key, pending state or RPC is created. Already retained
//!   sent payloads/receipts are not aborted, cleared, replayed or transferred.
//!   This is an explicit temporary feature limitation, not C3 write parity.
use crate::shell::Shell;
use vessel_app_kit::ui::Context;
use vessel_app_link::workspace::GitWrite;
use vessel_app_view::toasts::ToastTone;
use vessel_app_view::workspace::git::WriteIntent;

impl Shell {
    pub(super) fn git_write(
        &mut self,
        _write: GitWrite,
        _intent: WriteIntent,
        cx: &mut Context<'_, Self>,
    ) {
        self.git_unverified_view(cx);
    }

    pub(super) fn git_unverified_view(&mut self, cx: &mut Context<'_, Self>) {
        let words = self.words;
        self.toast(words.git_panel().unverified_view(), ToastTone::Info, cx);
    }
}

#[cfg(test)]
mod tests;
