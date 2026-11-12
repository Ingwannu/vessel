//! Automation (protocol 11, ADRs 0033 and 0034): schedules, saved
//! prompts, the starter told, webhooks. Each part is a module of its own;
//! the service's `automation` and `scheduled` files are their calls and
//! the runs they make through the service's checks.

pub(crate) mod error;
pub(crate) mod prompts;
pub(crate) mod schedules;
pub(crate) mod time;
pub(crate) mod turns;
