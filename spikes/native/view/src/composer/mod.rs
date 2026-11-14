//! What the session's composer means beyond the text it holds: the
//! controls of a running session (its model, effort, permissions and other
//! options changed while it runs). The composer's other rows (the agent's
//! `/` commands, attachments, `@` files, drafts and recall) join here as
//! their slices land.
//!
//! The drawing is the app's (`app/src/areas/composer/`); the rules are
//! here, with no UI toolkit in them, so the page, the desktop and the phone
//! decide the same way (ADR 0036).

pub mod modes;
pub mod running;
