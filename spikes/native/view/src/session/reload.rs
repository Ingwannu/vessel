//! Whether a tab menu offers "Reload agent" (TRACKER 803, ADR 0110): the
//! node's `session.reload`, Paseo's `refresh_agent_request`.
//!
//! [Decision Log]
//! - Purpose: Paseo offers Reload agent on every agent tab; a press while
//!   the host is disconnected says so ("Host is not connected") instead.
//! - Constraint: a node older than [`RELOAD_PROTOCOL`] has no such call,
//!   and an agent that said it cannot load a conversation is refused by the
//!   node (`unsupported`), which Paseo has no equivalent of.
//! - Choice (coordinator, 2026-11-18): hidden on a connected node older
//!   than the protocol and for an agent that cannot load its conversation,
//!   as the page's `canReload`; shown on a disconnected node, where a press
//!   says the host is not connected, as Paseo's.
//! - Cost: a node that connects later at an older protocol hides a row the
//!   person saw while it was away.

use vessel_proto::ProtocolVersion;
use vessel_proto::reload::RELOAD_PROTOCOL;

/// What the tab menu does with Reload agent for a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadOffer {
    /// Not offered: its node cannot reload, or its agent cannot load.
    Hidden,
    /// Offered; a press says the host is not connected.
    Disconnected,
    /// Offered; a press reloads.
    Ready,
}

/// Reload agent for a session on a node speaking `protocol` (`None`: not
/// connected), whose agent said it cannot load a conversation when
/// `cannot_load`.
#[must_use]
pub fn reload_offer(protocol: Option<ProtocolVersion>, cannot_load: bool) -> ReloadOffer {
    if cannot_load {
        return ReloadOffer::Hidden;
    }
    match protocol {
        None => ReloadOffer::Disconnected,
        Some(protocol) if RELOAD_PROTOCOL.is_some_and(|since| protocol >= since) => {
            ReloadOffer::Ready
        }
        Some(_) => ReloadOffer::Hidden,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offered_from_the_reload_protocol_and_while_disconnected_never_when_it_cannot_load() {
        let at = |version| Some(ProtocolVersion::new(version));
        assert_eq!(reload_offer(at(31), false), ReloadOffer::Ready);
        assert_eq!(reload_offer(at(32), false), ReloadOffer::Ready);
        assert_eq!(reload_offer(at(30), false), ReloadOffer::Hidden);
        assert_eq!(reload_offer(None, false), ReloadOffer::Disconnected);
        assert_eq!(reload_offer(at(31), true), ReloadOffer::Hidden);
        assert_eq!(reload_offer(None, true), ReloadOffer::Hidden);
    }
}
