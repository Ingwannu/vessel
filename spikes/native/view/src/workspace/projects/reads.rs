//! Project reads retain peer identity and order independently of mutation drafts.
//! A newer refresh or alias rebinding invalidates earlier responses before hydration.

use super::navigation::ProjectTarget;
use crate::machine_book::Name;
use std::collections::HashMap;
use vessel_proto::NodeId;

const MAX_PROJECT_READ_HOSTS: usize = 256;

/// Correlation for a single host's project list; never authorization by alias alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectReadKey {
    target: ProjectTarget,
    serial: u64,
}
impl ProjectReadKey {
    /// Exact peer captured before dispatch.
    #[must_use]
    pub const fn target(&self) -> &ProjectTarget {
        &self.target
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Pending,
    Settled,
}
#[derive(Debug)]
struct Read {
    key: ProjectReadKey,
    state: State,
    context: Option<crate::connection::ConnectionContext>,
}

/// Bounded read tickets survive form cancellation; forgetting a host releases its slot.
#[derive(Debug, Default)]
pub struct ProjectReads {
    serial: u64,
    hosts: HashMap<Name, Read>,
}
impl ProjectReads {
    /// Supersede an earlier read, or refuse before changing state at a lifetime bound.
    #[must_use]
    pub fn begin(&mut self, target: ProjectTarget) -> Option<ProjectReadKey> {
        if !self.hosts.contains_key(target.host()) && self.hosts.len() >= MAX_PROJECT_READ_HOSTS {
            return None;
        }
        // Never wrap: an old completion must not become the newest ticket again.
        let serial = self.serial.checked_add(1)?;
        let key = ProjectReadKey { target, serial };
        let _previous = self.hosts.insert(
            key.target.host().clone(),
            Read {
                key: key.clone(),
                state: State::Pending,
                context: None,
            },
        );
        self.serial = serial;
        Some(key)
    }

    /// Capture the shared admission; a same-node reconnect is a different cache.
    #[must_use]
    pub fn begin_on(
        &mut self,
        target: ProjectTarget,
        context: crate::connection::ConnectionContext,
    ) -> Option<ProjectReadKey> {
        if target.node() != context.node() || target.host().as_str() != context.machine().as_str() {
            return None;
        }
        let key = self.begin(target)?;
        self.hosts.get_mut(key.target.host())?.context = Some(context);
        Some(key)
    }

    /// Admission that actually produced the current list request.
    #[must_use]
    pub fn context(&self, host: &Name) -> Option<&crate::connection::ConnectionContext> {
        self.hosts.get(host)?.context.as_ref()
    }

    /// Only the current ticket settles; the app separately rechecks the online peer.
    #[must_use]
    pub fn finish(&mut self, key: &ProjectReadKey) -> bool {
        let Some(read) = self.hosts.get_mut(key.target.host()) else {
            return false;
        };
        if read.key != *key || read.state != State::Pending {
            return false;
        }
        read.state = State::Settled;
        true
    }

    /// Which authenticated peer owns the cache under this route alias.
    #[must_use]
    pub fn node(&self, host: &Name) -> Option<NodeId> {
        self.hosts.get(host).map(|read| read.key.target.node())
    }

    /// A fixed-target readiness query must wait for an unsettled authoritative refresh.
    #[must_use]
    pub fn pending(&self, host: &Name, node: NodeId) -> bool {
        self.hosts
            .get(host)
            .is_some_and(|read| read.key.target.node() == node && read.state == State::Pending)
    }

    /// Disconnect invalidates even same-node pending reads; reconnect must hydrate anew.
    pub fn retain_online(&mut self, targets: &[ProjectTarget]) {
        self.hosts.retain(|name, read| {
            targets
                .iter()
                .any(|target| target.host() == name && target.node() == read.key.target.node())
        });
    }

    /// Cancel hydration authority, without undoing any already sent node operation.
    pub fn detach(&mut self, host: &Name) {
        let _previous = self.hosts.remove(host);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(name: &str) -> ProjectTarget {
        ProjectTarget::new(Name::parse(name).unwrap(), NodeId::random())
    }

    #[test]
    fn refresh_rebinding_and_forget_revoke_old_reply_authority() {
        let mut reads = ProjectReads::default();
        let original = target("laptop");
        let old = reads.begin(original.clone()).unwrap();
        let newest = reads.begin(original.clone()).unwrap();
        assert!(!reads.finish(&old));
        assert!(reads.pending(original.host(), original.node()));
        assert!(reads.finish(&newest));
        assert!(!reads.finish(&newest));
        let rebound = target("laptop");
        let replacement = reads.begin(rebound.clone()).unwrap();
        assert_eq!(reads.node(rebound.host()), Some(rebound.node()));
        assert!(!reads.finish(&newest));
        reads.detach(rebound.host());
        assert!(!reads.finish(&replacement));
        let returned = reads.begin(original).unwrap();
        assert!(!reads.finish(&old));
        assert!(reads.finish(&returned));
    }

    #[test]
    fn reconnect_requires_a_new_read_even_when_the_node_identity_did_not_change() {
        let mut reads = ProjectReads::default();
        let target = target("laptop");
        let old = reads.begin(target.clone()).unwrap();
        reads.retain_online(&[]);
        assert_eq!(reads.node(target.host()), None);
        let current = reads.begin(target.clone()).unwrap();
        reads.retain_online(std::slice::from_ref(&target));
        assert!(!reads.finish(&old));
        assert!(reads.finish(&current));
    }

    #[test]
    fn exhaustion_and_capacity_preserve_existing_admission_and_never_evict() {
        let mut reads = ProjectReads::default();
        let first = target("first");
        let pending = reads.begin(first.clone()).unwrap();
        reads.serial = u64::MAX;
        assert!(reads.begin(first.clone()).is_none());
        assert!(reads.finish(&pending));
        reads.serial = 1;
        for n in 1..MAX_PROJECT_READ_HOSTS {
            let _key = reads.begin(target(&format!("host-{n}"))).unwrap();
        }
        assert!(reads.begin(target("extra")).is_none());
        assert_eq!(reads.node(first.host()), Some(first.node()));
        let existing = reads.begin(first).unwrap();
        assert!(reads.finish(&existing));
    }
}
