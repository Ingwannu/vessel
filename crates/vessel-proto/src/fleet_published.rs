//! What a machine shares through its fleet, for a person (protocol 16,
//! ADR 0056): names and metadata of the bundle a home publishes or a member
//! last received, never contents, secret values or tool servers' commands.
//!
//! [Decision Log]
//! - Purpose: a person's Settings › Sync page shows what a home shares; the
//!   only read of it was `fleet.manifest`, the member protocol (`sync`
//!   scope, which no device paired by a person holds), so the page asked
//!   with a member's call and was refused.
//! - Choice: a call of its own, `admin` as `fleet.members` and
//!   `secret.list` are; not a field on `fleet.status`, which stays cheap
//!   and readable by a read-only device while a published list can be
//!   long. Every role answers with a value: a standalone machine publishes
//!   nothing, which is not an error.
//! - Not forwardable: each machine is read over its own connection, and a
//!   home already hears its members' applied state (`fleet.members`,
//!   `fleet.status`); forwarding can be added later without breaking
//!   anyone, taking it away could not.

use serde::{Deserialize, Serialize};

use crate::{BundlePath, Generation, McpServerName, NodeId, SecretName};

/// `fleet.published`: what this machine shares, or last received, through
/// its fleet. Needs `admin`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GetPublished {}

/// The answer, by this machine's role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum FleetPublished {
    /// In no fleet: it publishes nothing and receives nothing.
    Standalone,
    /// A home: the bundle it publishes now; absent before its first one.
    Home {
        /// What it publishes.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bundle: Option<PublishedBundle>,
    },
    /// A member: the bundle it last received from `home`; absent before
    /// its first pull.
    Member {
        /// Where the bundle came from.
        home: NodeId,
        /// What it last received.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bundle: Option<PublishedBundle>,
        /// The version last applied here: older than `bundle`'s when the
        /// last apply did not finish, absent before the first.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        applied: Option<Generation>,
    },
    /// A role a newer node has; show nothing about it.
    #[serde(other)]
    Unknown,
}

/// A bundle by name: no file contents, no secret values, no commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PublishedBundle {
    /// Its version.
    pub generation: Generation,
    /// Every file, by path in the bundle's order.
    pub files: Vec<SharedFile>,
    /// The tool (MCP) servers, by name.
    pub servers: Vec<McpServerName>,
    /// The secrets those servers name, each once, sorted: names only; each
    /// machine resolves them from its own store.
    pub secrets: Vec<SecretName>,
}

/// One shared file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SharedFile {
    /// Its place in the bundle.
    pub path: BundlePath,
    /// What it is, from its place.
    pub kind: SharedFileKind,
    /// Its size in bytes.
    pub size: u64,
    /// Whether it is written executable (a skill's script).
    pub executable: bool,
}

/// What a shared file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum SharedFileKind {
    /// The instructions file.
    Instructions,
    /// A file of a skill.
    Skill,
    /// Anything else, a kind a newer node names included.
    #[serde(other)]
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Call, ProtocolVersion, Request as _, Scope};

    #[test]
    fn published_is_admin_not_forwarded_and_of_protocol_sixteen() {
        let call = GetPublished {}.into_call();
        let method = call.method();
        assert_eq!(
            method.scope().expect("legacy method requires a scope"),
            Scope::Admin
        );
        assert!(!method.forwardable());
        assert_eq!(call.since(), Some(ProtocolVersion::new(16)));
        let encoded = serde_json::to_string(&call).unwrap();
        assert_eq!(serde_json::from_str::<Call>(&encoded).unwrap(), call);
    }

    #[test]
    fn every_role_is_a_value_and_newer_roles_and_kinds_stay_readable() {
        let standalone = serde_json::to_string(&FleetPublished::Standalone).unwrap();
        assert_eq!(standalone, r#"{"role":"standalone"}"#);
        let member = FleetPublished::Member {
            home: NodeId::random(),
            bundle: Some(PublishedBundle {
                generation: Generation::new(3),
                files: vec![SharedFile {
                    path: "skills/build/run.sh".parse().unwrap(),
                    kind: SharedFileKind::Skill,
                    size: 12,
                    executable: true,
                }],
                servers: vec!["github".parse().unwrap()],
                secrets: vec!["GITHUB_TOKEN".parse().unwrap()],
            }),
            applied: Some(Generation::new(2)),
        };
        let encoded = serde_json::to_string(&member).unwrap();
        assert_eq!(
            serde_json::from_str::<FleetPublished>(&encoded).unwrap(),
            member
        );
        assert_eq!(
            serde_json::from_str::<FleetPublished>(r#"{"role":"relay_only","extra":1}"#).unwrap(),
            FleetPublished::Unknown
        );
        assert_eq!(
            serde_json::from_str::<SharedFileKind>(r#""command""#).unwrap(),
            SharedFileKind::Other
        );
    }
}
