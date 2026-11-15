//! Workspace label presentation and workflows; never session key/value metadata.
//! Catalog ownership stays host-local even when the picker merges online hosts.

mod catalog;
mod create;
mod manager;
mod picker;
mod replica;
mod values;

pub use catalog::{Catalog, Host, HostState, project};
pub use create::{CreateDraft, PickerPage};
pub use manager::{Draft, Manager, ManagerState};
pub use picker::{Picker, PickerRow};
pub use replica::Replica;
pub use values::{Assignment, Color, Count, Failure, Label, Name, NameError, Ticket, Tickets};

/// Identifies a connection through the existing validated node-book name.
pub use crate::machine_book::Name as HostKey;

#[cfg(test)]
mod tests;
