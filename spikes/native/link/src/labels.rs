//! Typed catalog UI seam. No RPC is invented before ADR 0086 is integrated.
//! Workspace assignment carries ADR 0082's reviewed identity; runtime stays pending.

use vessel_app_view::labels::{Catalog, Count, Failure, Label, Name, Ticket};

/// Intent issued by a workspace picker or catalog manager, with validated names.
#[derive(Debug, Clone)]
pub enum Command {
    /// Load the host's authoritative catalog.
    List,
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
    /// Authoritative definitions, or a typed refusal.
    Listed(Result<Catalog, Failure>),
    /// Assignment settlement retains the immutable workspace and requested name.
    Assigned {
        /// The registry workspace that owned the intent.
        workspace: vessel_proto::WorkspaceId,
        /// Original requested identity, regardless of canonical reply spelling.
        name: Name,
        /// Ordered registry observation owns committed assignment projection.
        result: Result<(), Failure>,
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
            Self::List => Answer::Listed(Err(failure)),
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

#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    pub(super) async fn labels(&self, request: Request) {
        // Even the pending-contract refusal retains exact admission semantics.
        let failure = match self.selected_connection(&request.context) {
            Ok(_original_connection) => Failure::Unsupported,
            Err(vessel_app_view::connection::Refusal::Changed) => Failure::Changed,
            Err(vessel_app_view::connection::Refusal::Closed) => Failure::Offline,
        };
        self.emit(crate::Event::Labels(request.refused(failure)))
            .await;
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
            Command::List.refused(Failure::Unsupported),
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
