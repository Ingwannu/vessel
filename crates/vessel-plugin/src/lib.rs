//! In-process create hooks (ADR 0068).
//!
//! Paseo's fourteen hook names live here so a fifteenth does not compile.
//! The node calls two of them, `agent.create.before` and
//! `agent.create.after`, on the shared session create. A release build
//! keeps the registry empty, so that create is unchanged. This crate
//! starts no process and installs nothing.

mod check;
mod error;
mod hook;
mod id;
mod registry;

pub use error::{HandlerStop, PluginError};
pub use hook::{Hook, Wait, WaitingHook};
pub use id::PluginId;
pub use registry::{AfterHandler, BeforeHandler, Idle, Registry};
