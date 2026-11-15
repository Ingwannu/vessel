//! Typed plugin catalog seam while node methods remain unassigned (ADR 0079).
//!
//! The real adapter replaces `UnavailableNode` only after the node service and
//! one shared pending method table exist. No invented RPC is sent here.

use std::future::Future;

use vessel_app_view::plugins::{CatalogTicket, Problem};
use vessel_proto::{NodeId, plugins::ClientCatalog};

/// The plugin catalog's node boundary is an actual open extension seam; the
/// native worker uses an in-memory source until its node implementation exists.
/// Generic dispatch keeps neither dynamic casts nor node internals in the app.
pub trait CatalogSource {
    /// Read one host's authoritative running installations.
    fn catalog(&self, node: NodeId) -> impl Future<Output = Result<ClientCatalog, Problem>> + Send;
}

/// Honest product behavior before the pending plugin protocol is assigned.
#[derive(Debug, Clone, Copy)]
pub struct UnavailableNode;

impl CatalogSource for UnavailableNode {
    fn catalog(
        &self,
        _node: NodeId,
    ) -> impl Future<Output = Result<ClientCatalog, Problem>> + Send {
        std::future::ready(Err(Problem::Unsupported))
    }
}

/// Feature commands stay out of the shared link's entry module until wiring.
#[derive(Debug, Clone, Copy)]
pub enum Command {
    /// A read owns the node and request correlation captured by view.
    ReadCatalog(CatalogTicket),
}

/// A returned result retains its original node/request even across reconnect.
#[derive(Debug)]
pub enum Event {
    /// The view applies the complete result only to the still-pending read.
    CatalogRead {
        /// Captured ownership of the request.
        ticket: CatalogTicket,
        /// Parsed catalog or a safe typed failure.
        result: Result<ClientCatalog, Problem>,
    },
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::time::Duration;

    use super::{CatalogSource, Command, Event, Problem};

    // The helper request budget in Paseo plugins/runtime.ts:41. A catalog
    // read also needs a finite link deadline while its adapter is staged.
    const CATALOG_READ_DEADLINE: Duration = Duration::from_secs(30);

    /// Run a typed read against its source; cancellation drops the source future.
    pub(super) async fn run(command: Command, source: &impl CatalogSource) -> Event {
        with_deadline(command, source, CATALOG_READ_DEADLINE).await
    }

    async fn with_deadline(
        command: Command,
        source: &impl CatalogSource,
        deadline: Duration,
    ) -> Event {
        match command {
            Command::ReadCatalog(ticket) => {
                let result = tokio::time::timeout(deadline, source.catalog(ticket.node()))
                    .await
                    .map_err(|_elapsed| Problem::Timeout)
                    .and_then(std::convert::identity);
                Event::CatalogRead { ticket, result }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::plugins::UnavailableNode;
        use vessel_app_view::plugins::{HostSupport, Registry};
        use vessel_proto::{NodeId, plugins::ClientCatalog};

        struct InMemory;
        impl CatalogSource for InMemory {
            fn catalog(
                &self,
                node: NodeId,
            ) -> impl Future<Output = Result<ClientCatalog, Problem>> + Send {
                std::future::ready(
                    ClientCatalog::new(node, vec![]).map_err(|_invalid| Problem::InvalidCatalog),
                )
            }
        }

        struct Pending;
        impl CatalogSource for Pending {
            async fn catalog(&self, _node: NodeId) -> Result<ClientCatalog, Problem> {
                std::future::pending().await
            }
        }

        #[tokio::test]
        async fn fake_and_pending_protocol_results_retain_the_same_request() {
            let mut registry = Registry::default();
            let ticket = registry
                .begin(NodeId::random(), HostSupport::Supported)
                .expect("read");
            let Event::CatalogRead {
                ticket: returned,
                result,
            } = run(Command::ReadCatalog(ticket), &InMemory).await;
            assert_eq!(returned, ticket);
            assert_eq!(
                result,
                ClientCatalog::new(ticket.node(), vec![])
                    .map_err(|_invalid| Problem::InvalidCatalog)
            );
            let Event::CatalogRead { result, .. } =
                run(Command::ReadCatalog(ticket), &UnavailableNode).await;
            assert_eq!(result, Err(Problem::Unsupported));
        }

        #[tokio::test]
        async fn a_source_that_never_answers_has_a_finite_typed_deadline() {
            let mut registry = Registry::default();
            let ticket = registry
                .begin(NodeId::random(), HostSupport::Supported)
                .expect("read");
            let Event::CatalogRead {
                ticket: returned,
                result,
            } = with_deadline(Command::ReadCatalog(ticket), &Pending, Duration::ZERO).await;
            assert_eq!(returned, ticket);
            assert_eq!(result, Err(Problem::Timeout));
        }
    }
}

impl Command {
    /// Resolve a feature command without using a fabricated node endpoint.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn run(self, source: &impl CatalogSource) -> Event {
        native::run(self, source).await
    }
}
