//! Where `origin` keeps a repository, as `git.status` says it (TRACKER 467
//! step B, ADR 0123): the host and path a forge's web pages are under, so
//! a client can link the workspace's branch or the shown file there
//! ("Open on GitHub").
//!
//! [Decision Log]
//! - Purpose: Paseo's open-in-editor list ends with the forge's page for
//!   the branch or the shown file (`planner.ts:147`), built from the
//!   checkout's `remoteUrl`, which is `origin`'s address
//!   (`checkout-git.ts:1387`).
//! - Constraint: an address may hold credentials
//!   (`https://user:token@host/...`), and a status reaches every client and
//!   agent tool that asks for one; a peer older than the field is not sent
//!   it.
//! - Alternatives: the raw address, as Paseo sends it (rejected: a token
//!   would travel with every status); a method of its own (rejected: Paseo
//!   has it on the status, and a client would ask twice for one view).
//! - Choice: the host and path only, read by the node from `git remote
//!   get-url origin`. [`OriginRepo::kind`] carries only what the node alone
//!   can know: a self-hosted GitHub it holds a token for, where Paseo's
//!   daemon asks `gh auth status --hostname`. Each client maps the public
//!   hosts itself, as Paseo's client does (`forge.ts:49`).
//! - Cost: one git run per status, and none for a peer below
//!   [`ORIGIN_LINK_PROTOCOL`] (the node does not read it for such a peer).

use serde::{Deserialize, Serialize};

use super::GitStatus;
use crate::ProtocolVersion;
use crate::forge::{ForgeHost, ForgeKind, RepoPath};

/// `git.status`'s `origin` (ADR 0123 step B), on the four replies that
/// carry a status (`git.status`, `git.stage`, `git.unstage`,
/// `git.discard`). Pending: no peer is served it until integration
/// assigns its number.
pub const ORIGIN_LINK_PROTOCOL: Option<ProtocolVersion> = Some(ProtocolVersion::new(42));

/// Whether a peer of `protocol` is served `origin`.
#[must_use]
pub fn serves_origin(protocol: ProtocolVersion) -> bool {
    served_from(protocol, ORIGIN_LINK_PROTOCOL)
}

fn served_from(protocol: ProtocolVersion, since: Option<ProtocolVersion>) -> bool {
    since.is_some_and(|since| protocol >= since)
}

/// Where `origin` keeps a repository.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct OriginRepo {
    /// The host, lower-cased; with its port only for an `http(s)` address
    /// on a port that is not the scheme's own.
    pub host: ForgeHost,
    /// `owner/name`, or nested groups; no `.git`.
    pub path: RepoPath,
    /// What the node knows the host to be (a self-hosted GitHub it holds a
    /// token for, or GitHub's own hosts); absent when it knows nothing a
    /// client could not read from the host itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<ForgeKind>,
}

impl GitStatus {
    /// This status as a peer of `protocol` reads it: without `origin`
    /// below [`ORIGIN_LINK_PROTOCOL`]; `None` when nothing changes, so a
    /// current peer costs no copy.
    #[must_use]
    pub fn for_protocol(&self, protocol: ProtocolVersion) -> Option<Self> {
        self.served_since(protocol, ORIGIN_LINK_PROTOCOL)
    }

    /// As [`for_protocol`](Self::for_protocol), with the field starting at
    /// `since` (tests give it a number while the constant is pending).
    fn served_since(
        &self,
        protocol: ProtocolVersion,
        since: Option<ProtocolVersion>,
    ) -> Option<Self> {
        if self.origin.is_none() || served_from(protocol, since) {
            return None;
        }
        let mut older = self.clone();
        older.origin = None;
        Some(older)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(origin: Option<OriginRepo>) -> GitStatus {
        GitStatus {
            branch: Some("main".parse().unwrap()),
            head: None,
            upstream: None,
            base: None,
            operation: None,
            files: Vec::new(),
            more: false,
            origin,
        }
    }

    fn origin() -> OriginRepo {
        OriginRepo {
            host: "github.com".parse().unwrap(),
            path: "owner/name".parse().unwrap(),
            kind: Some(ForgeKind::Github),
        }
    }

    #[test]
    fn a_pending_field_reaches_no_peer() {
        assert_eq!(ORIGIN_LINK_PROTOCOL, None);
        assert!(!serves_origin(crate::PROTOCOL_VERSION));
        let older = status(Some(origin()))
            .for_protocol(crate::PROTOCOL_VERSION)
            .unwrap();
        assert_eq!(older.origin, None);
        assert_eq!(status(None).for_protocol(crate::PROTOCOL_VERSION), None);
    }

    #[test]
    fn a_peer_from_the_number_on_reads_it_and_one_below_does_not() {
        let since = Some(ProtocolVersion::new(42));
        let with = status(Some(origin()));
        assert_eq!(with.served_since(ProtocolVersion::new(42), since), None);
        assert_eq!(with.served_since(ProtocolVersion::new(43), since), None);
        let below = with.served_since(ProtocolVersion::new(41), since).unwrap();
        assert_eq!(below, status(None));
    }

    #[test]
    fn the_wire_names_host_and_path_and_leaves_an_unknown_kind_out() {
        let json = serde_json::to_value(status(Some(OriginRepo {
            kind: None,
            ..origin()
        })))
        .unwrap();
        assert_eq!(
            json["origin"],
            serde_json::json!({ "host": "github.com", "path": "owner/name" })
        );
        let json = serde_json::to_value(status(None)).unwrap();
        assert!(json.get("origin").is_none());
        // An older node's status, without the field, still reads.
        let read: GitStatus = serde_json::from_str(r#"{"files":[],"more":false}"#).unwrap();
        assert_eq!(read.origin, None);
    }
}
