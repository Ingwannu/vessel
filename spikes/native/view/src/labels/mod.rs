//! Workspace label presentation and workflows; never session key/value metadata.
//! Catalog ownership stays host-local even when the picker merges online hosts.

mod binding;
mod catalog;
mod create;
mod manager;
mod picker;
mod registry;
mod replica;
mod search;
mod values;
mod workspace;

pub use binding::{Bindings, bind_hosts};
pub use catalog::{Catalog, Host, HostState, project};
pub use create::{CreateDraft, PickerPage};
pub use manager::{Draft, Manager, ManagerState};
pub use picker::{Picker, PickerRow};
pub use registry::Assignments;
pub use replica::Replica;
pub use values::{
    Assignment, Color, Count, EditAccess, Failure, Label, Name, NameError, Ticket, Tickets,
};
pub use workspace::WorkspaceTarget;

/// Catalog ownership follows the answered node identity, never a local alias.
pub use vessel_proto::NodeId as HostKey;

#[cfg(test)]
mod tests;
