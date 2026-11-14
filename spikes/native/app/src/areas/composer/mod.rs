//! The session's composer beyond the text it holds (track P): a running
//! session's controls, what each agent offers (its lists, kept and
//! refreshed behind the controls), its `/` commands menu, then attachments, `@` files,
//! drafts and recall as their slices land. The composer card itself, its
//! pills and the effort card are the draft's (`areas/draft/`), shared by
//! the new chat and a session; this area decides whose options they show
//! and what a choice does there.
//!
//! The rules are `vessel_app_view::composer`'s; this only wires them to
//! the shell, the link and the toasts.

pub(crate) mod modes;
pub(crate) mod offers;
pub(crate) mod running;
mod slash;

use std::cell::{OnceCell, RefCell};
use std::time::Instant;

pub(crate) use running::ControlsTarget;
pub(crate) use slash::{menu, text};
use vessel_app_kit::autocomplete::{AutocompleteState, Listing};
use vessel_app_view::composer::reveal::Reveal;
use vessel_app_view::composer::running::InFlight;
use vessel_app_view::composer::slash::Chosen;
use vessel_app_view::offers::Offers;

/// The composer's own state.
#[derive(Debug, Default)]
pub(crate) struct ComposerArea {
    /// A running session's option changes waiting for their answers.
    pub(crate) changes: InFlight,
    /// What each agent on each machine offers, as last known.
    pub(crate) offers: Offers,
    /// The `/` menu's chosen row.
    pub(crate) slash: Chosen,
    /// The `/` menu's scroll.
    pub(crate) list: AutocompleteState,
    /// How open the `/` menu is (it opens and closes over a glide; drawn
    /// with the window, so kept behind a cell).
    pub(crate) reveal: RefCell<Reveal>,
    /// The `/` menu's last rows and choice, drawn while it closes.
    pub(crate) last_menu: RefCell<Option<(Listing, Option<usize>)>>,
    /// When this area first measured time: the glides' clock.
    epoch: OnceCell<Instant>,
}

impl ComposerArea {
    /// Seconds since the area first measured time (the glides' clock).
    pub(crate) fn now(&self) -> f32 {
        self.epoch.get_or_init(Instant::now).elapsed().as_secs_f32()
    }
}
