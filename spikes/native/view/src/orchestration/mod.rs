//! Orchestration settings own typed host effects shared with onboarding.
//!
//! [Decision Log]
//! - Reuse the node's policy/selection values; a second settings model would drift.
//! - Keep target identity, visit and attempt together, so A-to-B-to-A navigation
//!   cannot apply an old acknowledgement to a new screen.
//! - Picker edits are local until Save. Removal requires a fresh node plan;
//!   controls never treat a timeout as proof that an effect did not happen.
mod contract;
mod load;
mod page;
mod picker;
pub use contract::{Action, Answer, Consumer, Output, Problem, Scope, Target, Task, Visit};
pub use load::Load;
pub use page::{Confirmation, Page, has_selection, maintenance};
pub use picker::Picker;
#[cfg(test)]
mod tests;
