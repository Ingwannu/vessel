//! Typed catalog UI seam. No RPC is invented before ADR 0086 is integrated.
//! Workspace assignment is added only with ADR 0082's reviewed `WorkspaceId`.

use vessel_app_view::labels::{Catalog, Count, Failure, Label, Name, Ticket};

/// Intent issued by the catalog manager, with validated names.
#[derive(Debug, Clone)]
pub enum Command {
    /// Load the host's authoritative catalog.
    List,
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
    pub const fn refused(&self, failure: Failure) -> Answer {
        match self {
            Self::List => Answer::Listed(Err(failure)),
            Self::Update { .. } => Answer::Updated(Err(failure)),
            Self::InspectDelete(_) => Answer::Inspected(Err(failure)),
            Self::Delete(_) => Answer::Deleted(Err(failure)),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    pub(super) async fn labels(
        &self,
        context: vessel_app_view::connection::ConnectionContext,
        ticket: Ticket,
        ask: Command,
    ) {
        // Even the pending-contract refusal retains exact admission semantics.
        let failure = match self.selected_connection(&context) {
            Ok(_original_connection) => Failure::Unsupported,
            Err(vessel_app_view::connection::Refusal::Changed) => Failure::Changed,
            Err(vessel_app_view::connection::Refusal::Closed) => Failure::Offline,
        };
        self.emit(crate::Event::Labels(Event {
            context,
            ticket,
            answer: ask.refused(failure),
        }))
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
