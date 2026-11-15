//! What the session's composer means beyond the text it holds: the
//! controls of a running session (its model, effort, permissions and other
//! options changed while it runs), the agent's `/` commands (`slash`,
//! ranked by `matching`), what was typed and not sent, kept per session
//! (`drafts`), Up and Down through the messages sent (`recall`), the
//! files attached to the next message (`attach`'s rules, `tray`'s state),
//! and the pictures of the images sent (`sent`).
//! The `@` files join here as their slice lands.
//!
//! The drawing is the app's (`app/src/areas/composer/`); the rules are
//! here, with no UI toolkit in them, so the page, the desktop and the phone
//! decide the same way (ADR 0036).

pub mod attach;
pub mod context;
pub mod drafts;
pub mod local;
pub mod matching;
pub mod mentions;
pub mod modes;
pub mod recall;
pub mod reveal;
pub mod running;
pub mod sent;
pub mod slash;
pub mod tray;
pub mod trays;
pub mod viewer;
