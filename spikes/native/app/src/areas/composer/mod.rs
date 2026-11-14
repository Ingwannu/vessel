//! The session's composer beyond the text it holds (track P): a running
//! session's controls, what each agent offers (its lists, kept and
//! refreshed behind the controls), then its `/` commands, attachments, `@` files,
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

pub(crate) use running::ControlsTarget;
use vessel_app_view::composer::running::InFlight;
use vessel_app_view::offers::Offers;

/// The composer's own state.
#[derive(Debug, Default)]
pub(crate) struct ComposerArea {
    /// A running session's option changes waiting for their answers.
    pub(crate) changes: InFlight,
    /// What each agent on each machine offers, as last known.
    pub(crate) offers: Offers,
}
