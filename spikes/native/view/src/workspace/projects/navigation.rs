//! Add Project navigation, following Paseo add-project-flow/model.ts.
//! No IO or RPC dispatch; a view target is not authority to mutate a peer.
//! The link still verifies its authenticated `NodeId` before any eventual call.

use crate::machine_book::Name;
use std::{collections::HashMap, path::PathBuf};
use vessel_proto::{NodeId, workspace::CloneUrl};

const MAX_FLOW_HISTORY: usize = 3;
const MAX_FLOW_DRAFTS: usize = 64;
const MAX_FLOW_INPUT_BYTES: usize = 64 * 1024;

/// A captured node-book placement, never inferred from a path or the first remote alias.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectTarget {
    host: Name,
    node: NodeId,
}
impl ProjectTarget {
    /// Capture the welcome identity the host chooser displayed.
    #[must_use]
    pub const fn new(host: Name, node: NodeId) -> Self {
        Self { host, node }
    }
    /// Exact selected book name.
    #[must_use]
    pub const fn host(&self) -> &Name {
        &self.host
    }
    /// Exact selected welcome identity.
    #[must_use]
    pub const fn node(&self) -> NodeId {
        self.node
    }
}

/// The row can distinguish an older host from an unavailable local-only bridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// Advertised by the exact currently connected peer or local bridge.
    Available,
    /// The peer must be updated before this method can be offered.
    UpdateRequired,
    /// This platform/host has no suitable local bridge.
    Unavailable,
}

/// Methods have distinct capability gates; registration alone does not permit all of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectMethods {
    /// Exact selected-root registration is required by every method.
    pub register: Availability,
    /// A desktop picker on an authenticated managed-local target only.
    pub browse: Availability,
    /// Host-side clone support; account search can have separate query state later.
    pub clone: Availability,
    /// Empty directory creation, distinct from legacy Git init.
    pub new_directory: Availability,
}

/// The caller projects online fleet entries; the model retains no second host registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectHost {
    /// The exact online identity, not merely its displayed label.
    pub target: ProjectTarget,
    /// Latest declared support for this host.
    pub methods: ProjectMethods,
}

/// Stable method identity; strings and translated row text do not choose actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddMethod {
    /// Search directories using the selected host's filesystem API.
    Directory,
    /// Request the local desktop directory picker.
    Browse,
    /// Search `GitHub` or select a full Git remote for cloning.
    Github,
    /// Create one empty directory under an existing host parent.
    NewDirectory,
}

/// Every page carries its original target rather than consulting Settings' current host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectPage {
    /// Host discovery/search, including zero hosts and late arrivals.
    Hosts {
        /// In-flow host-filter text, restored by Back.
        query: String,
    },
    /// Choose a supported method without any mutation.
    Methods {
        /// Placement captured by the chooser or initial preferred-host resolution.
        target: ProjectTarget,
    },
    /// Search directories on the selected node.
    Directory {
        /// Node on which directory lookup and registration will run.
        target: ProjectTarget,
        /// Search draft; it is not a canonical filesystem path.
        query: String,
    },
    /// `GitHub` account search and manual remote choices.
    Github {
        /// Node whose `GitHub` credentials/search are used.
        target: ProjectTarget,
        /// Account search or manual remote input draft.
        query: String,
    },
    /// Clone parent draft is retained separately for each target and remote.
    GithubLocation {
        /// Node that will clone the selected remote.
        target: ProjectTarget,
        /// Validated remote, also distinguishing this clone-location draft.
        remote: CloneUrl,
    },
    /// Choose an existing parent on the node.
    NewDirectoryParent {
        /// Node that owns the parent directory.
        target: ProjectTarget,
        /// Parent search draft, interpreted only by that node.
        query: String,
    },
    /// Name draft is retained separately for each target and parent.
    NewDirectoryName {
        /// Node that will validate and create the directory.
        target: ProjectTarget,
        /// Opaque node-local selected parent, used only as a draft cache key here.
        parent: PathBuf,
    },
}

/// Browsing is an explicit outgoing intent, not a side effect of navigation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectNavigation {
    /// Render the model's new current page.
    PageChanged,
    /// Resolve the managed-local bridge before opening an OS picker.
    Browse(ProjectTarget),
    /// Back from the first page closes the flow.
    Closed,
}

/// Refusals leave the existing draft and page intact.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NavigationRefusal {
    /// An event for an old/different page cannot skip required steps.
    #[error("this action does not apply to the current project page")]
    WrongPage,
    /// A forgotten/rebound/offline alias cannot silently retarget the draft.
    #[error("the selected project host is unavailable")]
    HostUnavailable,
    /// The method is disabled; no operation was dispatched.
    #[error("the project method is not available: {0:?}")]
    NotSupported(Availability),
    /// Bound all retained draft content, including server-supplied parent keys.
    #[error("the project draft exceeds its input limit")]
    InputLimit,
    /// Keep older drafts rather than evicting one behind the person's back.
    #[error("too many project drafts are retained in this flow")]
    DraftLimit,
    /// Defensive bound; valid page transitions need at most three previous pages.
    #[error("the project flow has too many pages")]
    DepthLimit,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum DraftKey {
    Directory(ProjectTarget, PathBuf),
    Clone(ProjectTarget, CloneUrl),
}

/// Page state and draft retention only; submission correlation remains `ProjectActions`'.
#[derive(Debug, Default)]
pub struct ProjectNavigationModel {
    current: Option<ProjectPage>,
    previous: Vec<ProjectPage>,
    drafts: HashMap<DraftKey, String>,
}
impl ProjectNavigationModel {
    /// Preferred or sole online host skips the host page, as in Paseo.
    #[must_use]
    pub fn open(hosts: &[ProjectHost], preferred: Option<&Name>) -> Self {
        Self {
            current: Some(initial_page(hosts, preferred)),
            ..Self::default()
        }
    }
    /// Closed has no current page; callers never need to index an empty stack.
    #[must_use]
    pub const fn page(&self) -> Option<&ProjectPage> {
        self.current.as_ref()
    }
    /// Discovery only resolves the initial host page. It never retargets later drafts.
    pub fn refresh_hosts(&mut self, hosts: &[ProjectHost], preferred: Option<&Name>) {
        if self.previous.is_empty() && matches!(self.current, Some(ProjectPage::Hosts { .. })) {
            let next = initial_page(hosts, preferred);
            if matches!(next, ProjectPage::Methods { .. }) {
                self.current = Some(next);
            }
        }
    }
    /// Choose a concrete currently online placement and retain the host search behind it.
    ///
    /// # Errors
    /// Wrong page, unavailable exact peer or exceeded depth.
    pub fn choose_host(
        &mut self,
        target: &ProjectTarget,
        hosts: &[ProjectHost],
    ) -> Result<ProjectNavigation, NavigationRefusal> {
        if !matches!(self.current, Some(ProjectPage::Hosts { .. })) {
            return Err(NavigationRefusal::WrongPage);
        }
        let target = online(target, hosts)?.target.clone();
        self.push(ProjectPage::Methods { target })
    }
    /// Recheck latest support against the captured welcome identity before advancing.
    ///
    /// # Errors
    /// Wrong page, unavailable peer/bridge, update required or exceeded depth.
    pub fn choose_method(
        &mut self,
        method: AddMethod,
        hosts: &[ProjectHost],
    ) -> Result<ProjectNavigation, NavigationRefusal> {
        let Some(ProjectPage::Methods { target }) = &self.current else {
            return Err(NavigationRefusal::WrongPage);
        };
        let host = online(target, hosts)?;
        available(host.methods.register)?;
        let target = target.clone();
        let query = String::new();
        let page = match method {
            AddMethod::Directory => ProjectPage::Directory { target, query },
            AddMethod::Browse => {
                available(host.methods.browse)?;
                return Ok(ProjectNavigation::Browse(target));
            }
            AddMethod::Github => {
                available(host.methods.clone)?;
                ProjectPage::Github { target, query }
            }
            AddMethod::NewDirectory => {
                available(host.methods.new_directory)?;
                ProjectPage::NewDirectoryParent { target, query }
            }
        };
        self.push(page)
    }
    /// Carry a validated remote choice, with no clone until explicit submission.
    ///
    /// # Errors
    /// Wrong page or exceeded depth.
    pub fn choose_repository(
        &mut self,
        remote: CloneUrl,
    ) -> Result<ProjectNavigation, NavigationRefusal> {
        let Some(ProjectPage::Github { target, .. }) = &self.current else {
            return Err(NavigationRefusal::WrongPage);
        };
        self.push(ProjectPage::GithubLocation {
            target: target.clone(),
            remote,
        })
    }
    /// Parent paths are node-local opaque values; the client does not canonicalize them.
    ///
    /// # Errors
    /// Wrong page, excessive retained key or exceeded depth.
    pub fn choose_parent(
        &mut self,
        parent: PathBuf,
    ) -> Result<ProjectNavigation, NavigationRefusal> {
        let Some(ProjectPage::NewDirectoryParent { target, .. }) = &self.current else {
            return Err(NavigationRefusal::WrongPage);
        };
        if parent.as_os_str().len() > MAX_FLOW_INPUT_BYTES {
            return Err(NavigationRefusal::InputLimit);
        }
        self.push(ProjectPage::NewDirectoryName {
            target: target.clone(),
            parent,
        })
    }
    /// The current text, including cached clone-parent/new-name drafts after Back.
    #[must_use]
    pub fn input(&self) -> Option<&str> {
        match self.current.as_ref()? {
            ProjectPage::Hosts { query }
            | ProjectPage::Directory { query, .. }
            | ProjectPage::Github { query, .. }
            | ProjectPage::NewDirectoryParent { query, .. } => Some(query),
            ProjectPage::Methods { .. } => None,
            page @ (ProjectPage::GithubLocation { .. } | ProjectPage::NewDirectoryName { .. }) => {
                draft_key(page).map(|key| self.drafts.get(&key).map_or("", String::as_str))
            }
        }
    }
    /// Retain text by exact host+node+parent/remote within this flow only.
    ///
    /// # Errors
    /// Wrong page, excessive input or exhausted retained-draft capacity.
    pub fn set_input(&mut self, text: String) -> Result<(), NavigationRefusal> {
        if text.len() > MAX_FLOW_INPUT_BYTES {
            return Err(NavigationRefusal::InputLimit);
        }
        let page = self.current.as_mut().ok_or(NavigationRefusal::WrongPage)?;
        match page {
            ProjectPage::Hosts { query }
            | ProjectPage::Directory { query, .. }
            | ProjectPage::Github { query, .. }
            | ProjectPage::NewDirectoryParent { query, .. } => *query = text,
            ProjectPage::Methods { .. } => return Err(NavigationRefusal::WrongPage),
            page @ (ProjectPage::GithubLocation { .. } | ProjectPage::NewDirectoryName { .. }) => {
                let key = draft_key(page).ok_or(NavigationRefusal::WrongPage)?;
                if !self.drafts.contains_key(&key) && self.drafts.len() >= MAX_FLOW_DRAFTS {
                    return Err(NavigationRefusal::DraftLimit);
                }
                drop(self.drafts.insert(key, text));
            }
        }
        Ok(())
    }
    /// Pop one page; Back from the initial page closes and releases retained drafts.
    #[must_use]
    pub fn back(&mut self) -> ProjectNavigation {
        self.current = self.previous.pop();
        if self.current.is_some() {
            ProjectNavigation::PageChanged
        } else {
            self.drafts.clear();
            ProjectNavigation::Closed
        }
    }
    fn push(&mut self, page: ProjectPage) -> Result<ProjectNavigation, NavigationRefusal> {
        if self.previous.len() >= MAX_FLOW_HISTORY {
            return Err(NavigationRefusal::DepthLimit);
        }
        let previous = self.current.take().ok_or(NavigationRefusal::WrongPage)?;
        self.current = Some(page);
        self.previous.push(previous);
        Ok(ProjectNavigation::PageChanged)
    }
}

fn initial_page(hosts: &[ProjectHost], preferred: Option<&Name>) -> ProjectPage {
    let preferred = preferred.and_then(|name| hosts.iter().find(|host| host.target.host() == name));
    let only_host = if hosts.len() == 1 {
        hosts.first()
    } else {
        None
    };
    let selected = preferred.or(only_host);
    match selected {
        Some(host) => ProjectPage::Methods {
            target: host.target.clone(),
        },
        None => ProjectPage::Hosts {
            query: String::new(),
        },
    }
}
fn online<'a>(
    target: &ProjectTarget,
    hosts: &'a [ProjectHost],
) -> Result<&'a ProjectHost, NavigationRefusal> {
    hosts
        .iter()
        .find(|host| &host.target == target)
        .ok_or(NavigationRefusal::HostUnavailable)
}
fn available(availability: Availability) -> Result<(), NavigationRefusal> {
    match availability {
        Availability::Available => Ok(()),
        refused @ (Availability::Unavailable | Availability::UpdateRequired) => {
            Err(NavigationRefusal::NotSupported(refused))
        }
    }
}
fn draft_key(page: &ProjectPage) -> Option<DraftKey> {
    match page {
        ProjectPage::GithubLocation { target, remote } => {
            Some(DraftKey::Clone(target.clone(), remote.clone()))
        }
        ProjectPage::NewDirectoryName { target, parent } => {
            Some(DraftKey::Directory(target.clone(), parent.clone()))
        }
        ProjectPage::Hosts { .. }
        | ProjectPage::Methods { .. }
        | ProjectPage::Directory { .. }
        | ProjectPage::Github { .. }
        | ProjectPage::NewDirectoryParent { .. } => None,
    }
}

#[cfg(test)]
mod tests;
