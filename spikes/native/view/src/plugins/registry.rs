//! The selected node and helper generation remain authoritative while offline.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

use crate::devices::{Ticket, Tickets};
use vessel_proto::plugins::{
    ClientDocument, Contribution, ContributionAddress, PluginAddress, PluginId, RuntimeGeneration,
    ThemeContribution,
};
use vessel_proto::{ErrorCode, NodeId};

const MAX_HOSTS: usize = 128;
const MAX_INSTALLATIONS: usize = 128;
const MAX_THEMES: usize = 256;

/// Safe failure classes; a helper's diagnostic text is never a translated UI error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Problem {
    /// The captured host has no current connection.
    #[error("the plugin host is offline")]
    Offline,
    /// The pending protocol or node capability is unavailable.
    #[error("this node does not support native plugins")]
    Unsupported,
    /// Ordinary node authorization refused the call.
    #[error("the node refused the plugin operation ({0:?})")]
    Refused(ErrorCode),
    /// A generation or request no longer belongs to the current installation.
    #[error("the plugin installation or request changed")]
    Stale,
    /// Host ownership and uniqueness must hold before replacing any snapshot.
    #[error("the plugin catalog is invalid")]
    InvalidCatalog,
    /// The captured contribution no longer exists in its own namespace.
    #[error("the plugin contribution is unavailable")]
    MissingContribution,
    /// Context must agree with the actual panel declaration, never its title.
    #[error("the plugin contribution does not support this context")]
    Context,
    /// Refuse new retained state rather than silently evicting a selected host.
    #[error("the plugin catalog exceeds {0}")]
    Capacity(CatalogBudget),
    /// Refreshes are serialized per node; queued invalidations coalesce.
    #[error("the plugin catalog is already refreshing")]
    Busy,
    /// The link queue could not accept an action; nothing was sent.
    #[error("the plugin request queue is full")]
    QueueFull,
    /// Every node read has a finite caller-owned deadline.
    #[error("the plugin request timed out")]
    Timeout,
    /// Existing checked ticket allocation refuses exhaustion without reuse.
    #[error("plugin request identity allocation failed")]
    Tickets(#[source] crate::devices::Problem),
}

/// Distinguishes which retained resource was refused, with its actual bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogBudget {
    /// Too many retained nodes.
    Hosts,
    /// Too many plugins on one node.
    Installations,
    /// Too many themes across one node's plugins.
    Themes,
}

impl fmt::Display for CatalogBudget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hosts => write!(f, "{MAX_HOSTS} hosts"),
            Self::Installations => write!(f, "{MAX_INSTALLATIONS} installations per host"),
            Self::Themes => write!(f, "{MAX_THEMES} themes per host"),
        }
    }
}

/// Capability comes from the node handshake, independently of connection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostSupport {
    /// The node speaks the assigned native-plugin contract.
    Supported,
    /// Old or unassigned protocol; previously retained contributions are withdrawn.
    Unsupported,
}

/// The captured node and read id cannot be replaced by a book label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CatalogTicket {
    node: NodeId,
    request: Ticket,
    owner: RegistryGeneration,
}

impl CatalogTicket {
    /// The node a link operation must address.
    #[must_use]
    pub const fn node(self) -> NodeId {
        self.node
    }
}

/// One validated immutable declaration graph and its exact node lifetime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installation {
    address: PluginAddress,
    document: Arc<ClientDocument>,
}

impl Installation {
    /// Preserve one helper lifetime and refuse duplicate or excessive themes.
    ///
    /// # Errors
    /// Refuses duplicate theme identities or a retained-theme budget overflow.
    pub fn new(address: PluginAddress, themes: Vec<ThemeContribution>) -> Result<Self, Problem> {
        if themes.len() > MAX_THEMES {
            return Err(Problem::Capacity(CatalogBudget::Themes));
        }
        let document = ClientDocument::new(vec![], vec![], themes)
            .map_err(|_invalid| Problem::InvalidCatalog)?;
        Ok(Self::from_document(address, document))
    }

    /// Accept only an already validated graph; no raw manifest crosses feature
    /// adapters. Whole-host ownership and budgets are checked during replacement.
    #[must_use]
    pub fn from_document(address: PluginAddress, document: ClientDocument) -> Self {
        Self {
            address,
            document: Arc::new(document),
        }
    }

    /// Node and helper generation captured by controls and actions.
    #[must_use]
    pub const fn address(&self) -> &PluginAddress {
        &self.address
    }

    /// Already validated palette values; appearance owns dedup and preference fallback.
    #[must_use]
    pub fn themes(&self) -> &[ThemeContribution] {
        self.document.themes()
    }

    /// Feature owners project this one graph into their existing registries.
    #[must_use]
    pub fn document(&self) -> &ClientDocument {
        &self.document
    }

    /// Internal immutable origin binding; requests share the checked allocation.
    pub(super) fn document_snapshot(&self) -> Arc<ClientDocument> {
        Arc::clone(&self.document)
    }

    pub(super) fn matches_document(&self, snapshot: &Arc<ClientDocument>) -> bool {
        Arc::ptr_eq(&self.document, snapshot) || self.document.as_ref() == snapshot.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Connection {
    Online,
    Offline,
}

#[derive(Debug)]
struct Host {
    connection: Connection,
    pending: Option<Ticket>,
    installations: Arc<[Installation]>,
    failure: Option<Problem>,
}

/// A stale response is observable to its link caller without changing current state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotApplied {
    /// The complete validated snapshot replaced that node's retained catalog.
    Applied,
    /// Disconnect, removal or a later refresh invalidated this reply.
    Stale,
}

/// Per-node retained catalogs; no global plugin-name map can redirect an action.
#[derive(Debug)]
pub struct Registry {
    generation: RegistryGeneration,
    tickets: Tickets,
    hosts: BTreeMap<NodeId, Host>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RegistryGeneration(RuntimeGeneration);

impl Default for Registry {
    fn default() -> Self {
        Self {
            generation: RegistryGeneration(RuntimeGeneration::random()),
            tickets: Tickets::default(),
            hosts: BTreeMap::new(),
        }
    }
}

impl Registry {
    /// Start one read; unsupported capability withdraws that host immediately.
    ///
    /// # Errors
    /// Refuses unsupported nodes, overlapping reads and the retained-host bound.
    pub fn begin(&mut self, node: NodeId, support: HostSupport) -> Result<CatalogTicket, Problem> {
        if support == HostSupport::Unsupported {
            drop(self.hosts.remove(&node));
            return Err(Problem::Unsupported);
        }
        if !self.hosts.contains_key(&node) && self.hosts.len() >= MAX_HOSTS {
            return Err(Problem::Capacity(CatalogBudget::Hosts));
        }
        if self
            .hosts
            .get(&node)
            .is_some_and(|host| host.pending.is_some())
        {
            return Err(Problem::Busy);
        }
        let request = self.tickets.next_ticket().map_err(Problem::Tickets)?;
        let host = self.hosts.entry(node).or_insert_with(|| Host {
            connection: Connection::Online,
            pending: None,
            installations: Arc::from([]),
            failure: None,
        });
        host.connection = Connection::Online;
        host.pending = Some(request);
        Ok(CatalogTicket {
            node,
            request,
            owner: self.generation,
        })
    }

    /// Validate ownership and uniqueness before replacing the whole host snapshot.
    ///
    /// # Errors
    /// Refuses malformed catalogs while retaining the previous safe snapshot.
    pub fn apply(
        &mut self,
        ticket: CatalogTicket,
        result: Result<Vec<Installation>, Problem>,
    ) -> Result<SnapshotApplied, Problem> {
        if ticket.owner != self.generation {
            return Ok(SnapshotApplied::Stale);
        }
        let Some(host) = self.hosts.get_mut(&ticket.node) else {
            return Ok(SnapshotApplied::Stale);
        };
        if host.pending != Some(ticket.request) {
            return Ok(SnapshotApplied::Stale);
        }
        host.pending = None;
        let validated = result.and_then(|entries| {
            validate_snapshot(ticket.node, &entries)?;
            validate_generations(&host.installations, &entries)?;
            Ok(entries)
        });
        match validated {
            Ok(entries) => {
                host.installations = entries.into();
                host.failure = None;
                Ok(SnapshotApplied::Applied)
            }
            Err(problem) => {
                host.failure = Some(problem);
                if problem == Problem::Offline {
                    host.connection = Connection::Offline;
                }
                Err(problem)
            }
        }
    }

    /// A lost connection retains rendering data but invalidates its in-flight read.
    pub fn offline(&mut self, node: NodeId) {
        if let Some(host) = self.hosts.get_mut(&node) {
            host.connection = Connection::Offline;
            host.pending = None;
        }
    }

    /// A forgotten host withdraws every contribution and cancels read ownership.
    pub fn remove(&mut self, node: NodeId) {
        drop(self.hosts.remove(&node));
    }

    /// Retained data is available offline for rendering; this never authorizes an action.
    #[must_use]
    pub fn installed(&self, node: NodeId) -> Option<&[Installation]> {
        self.hosts
            .get(&node)
            .map(|host| host.installations.as_ref())
    }

    /// Last read failure is independent of retained valid rendering data.
    #[must_use]
    pub fn failure(&self, node: NodeId) -> Option<Problem> {
        self.hosts.get(&node).and_then(|host| host.failure)
    }

    /// Revalidate the exact captured installation immediately before dispatch.
    ///
    /// # Errors
    /// Never substitutes another host; refuses offline, removed or replaced helpers.
    pub fn resolve_action(&self, address: &PluginAddress) -> Result<&Installation, Problem> {
        let host = self.hosts.get(&address.node).ok_or(Problem::Stale)?;
        if host.connection == Connection::Offline {
            return Err(Problem::Offline);
        }
        host.installations
            .iter()
            .find(|entry| entry.address == *address)
            .ok_or(Problem::Stale)
    }

    /// Resolve a current contribution on its captured node and generation.
    /// Retained offline declarations remain display data, never action authority.
    ///
    /// # Errors
    /// Refuses offline/stale installations or a missing exact kind/id pair.
    pub fn resolve_contribution(
        &self,
        address: &ContributionAddress,
    ) -> Result<&Contribution, Problem> {
        self.resolve_action(&address.installation)?
            .document()
            .contributions()
            .iter()
            .find(|item| item.id == address.contribution && item.content.kind() == address.kind)
            .ok_or(Problem::MissingContribution)
    }
}

fn validate_snapshot(node: NodeId, entries: &[Installation]) -> Result<(), Problem> {
    if entries.len() > MAX_INSTALLATIONS {
        return Err(Problem::Capacity(CatalogBudget::Installations));
    }
    let mut ids = BTreeSet::<&PluginId>::new();
    let mut themes = 0_usize;
    for entry in entries {
        if entry.address.node != node || !ids.insert(&entry.address.plugin) {
            return Err(Problem::InvalidCatalog);
        }
        // The budget covers the whole host, preventing many individually valid
        // installations from exceeding the picker/catalog memory envelope.
        themes = themes.saturating_add(entry.themes().len());
        if themes > MAX_THEMES {
            return Err(Problem::Capacity(CatalogBudget::Themes));
        }
    }
    Ok(())
}

fn validate_generations(
    previous: &[Installation],
    incoming: &[Installation],
) -> Result<(), Problem> {
    for entry in incoming {
        if previous
            .iter()
            .any(|old| old.address == entry.address && old.document != entry.document)
        {
            // Semantic changes must rotate the node-owned generation. Otherwise
            // an old captured action would silently acquire new behavior.
            return Err(Problem::InvalidCatalog);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
