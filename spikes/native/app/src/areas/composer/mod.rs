//! The composers beyond the text they hold (track P): a running
//! session's controls, what each agent offers (its lists, kept and
//! refreshed behind the controls), the `/` commands menu over the session's
//! composer and the new chat's, drafts and recall; attachments and `@`
//! files as their slices land. The composer card itself, its
//! pills and the effort card are the draft's (`areas/draft/`), shared by
//! the new chat and a session; this area decides whose options they show
//! and what a choice does there.
//!
//! The rules are `vessel_app_view::composer`'s; this only wires them to
//! the shell, the link and the toasts.

mod drafts;
pub(crate) mod modes;
pub(crate) mod offers;
pub(crate) mod running;
mod slash;

use std::cell::OnceCell;
use std::time::Instant;

pub(crate) use drafts::keep_drafts;
pub(crate) use running::ControlsTarget;
pub(crate) use slash::{draft_menu, draft_text, menu, text};
use vessel_app_view::composer::drafts::Drafts;
use vessel_app_view::composer::recall::Recall;
use vessel_app_view::composer::running::InFlight;
use vessel_app_view::offers::Offers;

/// The composer's own state.
#[derive(Debug, Default)]
pub(crate) struct ComposerArea {
    /// A running session's option changes waiting for their answers.
    pub(crate) changes: InFlight,
    /// What each agent on each machine offers, as last known.
    pub(crate) offers: Offers,
    /// The `/` menu over the session's composer.
    session_slash: slash::SlashMenu,
    /// The `/` menu over the new chat's composer: its own choice, scroll
    /// and glide, so one composer's menu never moves the other's.
    draft_slash: slash::SlashMenu,
    /// When this area first measured time: the glides' clock.
    epoch: OnceCell<Instant>,
    /// What was typed and not sent, per session and for the new chat.
    pub(crate) drafts: Drafts,
    /// A write of the drafts is waiting for typing to settle.
    pub(crate) saving: bool,
    /// The sent message Up and Down show in the session composer, if one.
    pub(crate) recall: Recall,
}

impl ComposerArea {
    /// Seconds since the area first measured time (the glides' clock).
    pub(crate) fn now(&self) -> f32 {
        self.epoch.get_or_init(Instant::now).elapsed().as_secs_f32()
    }
}
