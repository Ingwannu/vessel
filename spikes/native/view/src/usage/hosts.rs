//! Usage host selection persists identity, never a host's mutable display label.
use vessel_proto::NodeId;

/// Offline hosts remain visible in settings, but cannot feed the sidebar summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostAvailability {
    /// A real handshake/connection is available.
    Connected,
    /// The host is not connected.
    Disconnected,
}

/// The call boundary determines actual method support, without guessing a version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostSupport {
    /// The host cannot answer usage calls.
    Unsupported,
    /// Saved-account limits are available, but generic source reports are not.
    Limits,
    /// The reviewed source/account report API is available.
    Reports,
}

/// A selection surface needs identity and availability, not credentials or addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageHost {
    /// Node handshake identity survives a human-label change.
    pub node: NodeId,
    /// Presentation only, never used for matching or persistence.
    pub label: String,
    /// Current connection fact.
    pub availability: HostAvailability,
    /// Actual report support known by the node boundary.
    pub support: HostSupport,
}
impl UsageHost {
    fn reporting(&self) -> bool {
        self.availability == HostAvailability::Connected && self.support != HostSupport::Unsupported
    }
}

/// The saved pick takes precedence over the host of the active conversation.
#[derive(Debug)]
pub struct HostChoice<'a> {
    /// Explicit persistent user choice.
    pub picked: Option<NodeId>,
    /// Host of the current workspace/conversation.
    pub active: Option<NodeId>,
    /// Stable node-book order supplied by link.
    pub hosts: &'a [UsageHost],
}
impl HostChoice<'_> {
    /// Only connected reporting hosts can supply summary windows.
    #[must_use]
    pub fn sidebar(&self) -> Option<NodeId> {
        let reporting = |key| {
            self.hosts
                .iter()
                .find(|host| Some(host.node) == key && host.reporting())
        };
        reporting(self.picked)
            .or_else(|| reporting(self.active))
            .or_else(|| self.hosts.iter().find(|host| host.reporting()))
            .map(|host| host.node)
    }
    /// A picked connected old host stays selected so the modal can explain support.
    #[must_use]
    pub fn modal(&self) -> Option<NodeId> {
        self.hosts
            .iter()
            .find(|host| {
                Some(host.node) == self.picked && host.availability == HostAvailability::Connected
            })
            .map(|host| host.node)
            .or_else(|| self.sidebar())
            .or_else(|| {
                self.hosts
                    .iter()
                    .find(|host| host.availability == HostAvailability::Connected)
                    .map(|host| host.node)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn host(id: &str, availability: HostAvailability, support: HostSupport) -> UsageHost {
        UsageHost {
            node: id.parse().unwrap(),
            label: "Same label".to_owned(),
            availability,
            support,
        }
    }
    #[test]
    fn sidebar_uses_picked_then_active_then_first_reporting_identity() {
        let a = host(
            "00000000-0000-0000-0000-000000000001",
            HostAvailability::Connected,
            HostSupport::Reports,
        );
        let b = host(
            "00000000-0000-0000-0000-000000000002",
            HostAvailability::Connected,
            HostSupport::Limits,
        );
        let off = host(
            "00000000-0000-0000-0000-000000000003",
            HostAvailability::Disconnected,
            HostSupport::Reports,
        );
        let hosts = [a.clone(), b.clone(), off.clone()];
        assert_eq!(
            HostChoice {
                picked: Some(b.node),
                active: Some(a.node),
                hosts: &hosts
            }
            .sidebar(),
            Some(b.node)
        );
        assert_eq!(
            HostChoice {
                picked: Some(off.node),
                active: Some(b.node),
                hosts: &hosts
            }
            .sidebar(),
            Some(b.node)
        );
        assert_eq!(
            HostChoice {
                picked: Some(off.node),
                active: None,
                hosts: &hosts
            }
            .sidebar(),
            Some(a.node)
        );
    }
    #[test]
    fn modal_keeps_a_picked_old_host_while_sidebar_uses_a_reporting_one() {
        let old = host(
            "00000000-0000-0000-0000-000000000001",
            HostAvailability::Connected,
            HostSupport::Unsupported,
        );
        let current = host(
            "00000000-0000-0000-0000-000000000002",
            HostAvailability::Connected,
            HostSupport::Reports,
        );
        let hosts = [old.clone(), current.clone()];
        let choice = HostChoice {
            picked: Some(old.node),
            active: Some(current.node),
            hosts: &hosts,
        };
        assert_eq!(choice.modal(), Some(old.node));
        assert_eq!(choice.sidebar(), Some(current.node));
        assert!(
            HostChoice {
                picked: None,
                active: None,
                hosts: &[]
            }
            .modal()
            .is_none()
        );
    }
    #[test]
    fn a_display_rename_does_not_change_the_persisted_selection() {
        let mut host = host(
            "00000000-0000-0000-0000-000000000001",
            HostAvailability::Connected,
            HostSupport::Reports,
        );
        let picked = host.node;
        host.label = "A renamed host".to_owned();
        assert_eq!(
            HostChoice {
                picked: Some(picked),
                active: None,
                hosts: &[host]
            }
            .modal(),
            Some(picked)
        );
    }
}
