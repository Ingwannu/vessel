//! The workspace labels' commands and answers between the app and the
//! link (ADR 0086, protocol 28). Assignment carries ADR 0082's registry
//! identity; the calls themselves are `run`.

use vessel_app_view::labels::{Count, Failure, Label, Name, Ticket};
use vessel_proto::registry::{RegistryRevision, RegistrySnapshot};
use vessel_proto::workspace_labels::{CatalogCursor, CatalogList, CatalogRead};

#[cfg(not(target_arch = "wasm32"))]
mod run;

/// Intent issued by a workspace picker or catalog manager, with validated names.
#[derive(Debug, Clone)]
pub enum Command {
    /// Read the host's catalog: a snapshot without `cursor`, else the
    /// changes after it; `Observe` waits for the next change.
    List {
        /// Where the caller's copy stands.
        cursor: Option<CatalogCursor>,
        /// At once, or on the next change (or the node's heartbeat).
        read: CatalogRead,
    },
    /// Set one workspace assignment; creation and assignment are one operation.
    Assign {
        /// Registry identity captured with the exact admission.
        workspace: vessel_proto::WorkspaceId,
        /// Host definition, or a create draft whose default is violet.
        label: Label,
        /// The explicit desired state; never invert again at dispatch time.
        assignment: vessel_app_view::labels::Assignment,
    },
    /// Save name and color atomically; never two sequential writes.
    Update {
        /// Immutable target name.
        original: Name,
        /// Both replacement fields together.
        replacement: Label,
    },
    /// Read the host's workspace registry (its sessions' workspaces and
    /// the workspaces' labels): at once without `changed_from`, else when
    /// it moves past that revision (or at the node's heartbeat).
    Registry {
        /// The revision the caller holds.
        changed_from: Option<RegistryRevision>,
    },
    /// Read affected count before presenting confirmation.
    InspectDelete(Name),
    /// Delete after inspection and explicit user confirmation.
    Delete(Name),
}

/// One immutable UI request, keeping global dispatch a registration only.
#[derive(Debug, Clone)]
pub struct Request {
    /// Exact authenticated admission, never an alias lookup by node id.
    pub context: vessel_app_view::connection::ConnectionContext,
    /// Window-owned operation identity.
    pub ticket: Ticket,
    /// Validated semantic operation.
    pub ask: Command,
}

impl Request {
    /// Local refusals retain the same target, ticket and answer kind everywhere.
    #[must_use]
    pub fn refused(self, failure: Failure) -> Event {
        Event {
            context: self.context,
            ticket: self.ticket,
            answer: self.ask.refused(failure),
        }
    }
}

/// Answers retain host and ticket even on a local refusal.
#[derive(Debug, Clone)]
pub struct Event {
    /// The owning host.
    pub context: vessel_app_view::connection::ConnectionContext,
    /// The owning request.
    pub ticket: Ticket,
    /// Only semantic results cross into the UI.
    pub answer: Answer,
}

/// Disjoint result types prevent treating an inspection as a completed deletion.
#[derive(Debug, Clone)]
pub enum Answer {
    /// The host's catalog or its changes, with the head to continue from.
    Listed(Result<CatalogList, Failure>),
    /// The host's workspace registry.
    Registry(Result<Box<RegistrySnapshot>, Failure>),
    /// Assignment settlement retains the immutable workspace and requested name.
    Assigned {
        /// The registry workspace that owned the intent.
        workspace: vessel_proto::WorkspaceId,
        /// Original requested identity, regardless of canonical reply spelling.
        name: Name,
        /// The workspace's committed labels, in order; the registry's next
        /// snapshot carries the same.
        result: Result<Vec<Name>, Failure>,
    },
    /// A committed atomic update.
    Updated(Result<Label, Failure>),
    /// Advisory count before confirmation.
    Inspected(Result<Count, Failure>),
    /// Committed affected count.
    Deleted(Result<Count, Failure>),
}

impl Command {
    /// Settle an unavailable operation, keeping its precise result kind.
    #[must_use]
    pub fn refused(&self, failure: Failure) -> Answer {
        match self {
            Self::List { .. } => Answer::Listed(Err(failure)),
            Self::Registry { .. } => Answer::Registry(Err(failure)),
            Self::Assign {
                workspace, label, ..
            } => Answer::Assigned {
                workspace: *workspace,
                name: label.name.clone(),
                result: Err(failure),
            },
            Self::Update { .. } => Answer::Updated(Err(failure)),
            Self::InspectDelete(_) => Answer::Inspected(Err(failure)),
            Self::Delete(_) => Answer::Deleted(Err(failure)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_refusals_preserve_operation_kinds() {
        let name = "Ready".parse::<Name>().unwrap();
        let label = Label {
            name: name.clone(),
            color: vessel_app_view::labels::Color::Violet,
        };
        assert!(matches!(
            Command::List {
                cursor: None,
                read: CatalogRead::Now,
            }
            .refused(Failure::Unsupported),
            Answer::Listed(Err(Failure::Unsupported))
        ));
        let workspace = vessel_proto::WorkspaceId::random();
        assert!(
            matches!(Command::Assign {workspace, label: label.clone(), assignment: vessel_app_view::labels::Assignment::Assigned}.refused(Failure::Unsupported), Answer::Assigned {workspace: actual, name: original, result: Err(Failure::Unsupported)} if actual == workspace && original == name)
        );
        assert!(matches!(
            Command::Update {
                original: name.clone(),
                replacement: label
            }
            .refused(Failure::Unsupported),
            Answer::Updated(Err(Failure::Unsupported))
        ));
        assert!(matches!(
            Command::InspectDelete(name.clone()).refused(Failure::Unsupported),
            Answer::Inspected(Err(Failure::Unsupported))
        ));
        assert!(matches!(
            Command::Delete(name).refused(Failure::Unsupported),
            Answer::Deleted(Err(Failure::Unsupported))
        ));
    }
}
