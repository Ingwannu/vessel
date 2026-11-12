//! What an agent says of itself in `initialize`, and why a handshake was
//! refused: the facts `agent.status` reports (protocol 11, ADR 0032),
//! read without a turn.
//!
//! [Decision Log]
//! - Purpose: a person sees whether an agent is signed in, and which
//!   version it is, before a session fails on it. ACP says both in the
//!   handshake: `initialize` names the agent (`agentInfo`) and the ways it
//!   signs in (`authMethods`), and `session/new` answers `auth_required`
//!   (-32000) when it needs a sign-in first.
//! - Constraint: the handshake is the session's and the probe's; a status
//!   check must not start a turn (a turn costs the owner money), and the
//!   driver reports only what a session shows.
//! - Choice: the driver reports an [`Introduction`] once, right after
//!   `initialize` (`AgentOutput::Introduced`, which a session ignores and
//!   a probe keeps), and a failed handshake carries a typed [`Refused`]
//!   beside its message (another ACP version; a sign-in asked for), so a
//!   caller never reads a message to learn either.
//! - Bounds: the agent's texts are held to the wire's rules: a version or
//!   a method id that breaks them is left out, a method's name and
//!   description are cut (`AuthMethod::MAX_NAME`, `MAX_DESCRIPTION`) and
//!   their control characters dropped, at most `AuthMethod::MAX` methods
//!   are kept, and only methods of type `agent` (Vessel does not
//!   advertise ACP's terminal sign-in).

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::v1::schema as acp;
use vessel_proto::agents::{AgentVersion, AuthMethod, AuthMethodId};

/// What the agent said of itself in `initialize`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Introduction {
    /// `agentInfo.version`, when it is there and fits `AgentVersion`.
    pub version: Option<AgentVersion>,
    /// Its `authMethods` of type `agent`, each within the wire's bounds.
    pub methods: Vec<AuthMethod>,
}

/// Why the handshake failed, at the step Vessel can tell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// It speaks another ACP version than Vessel ([`SPEAKS`]).
    Version {
        /// The version it answered.
        speaks: u16,
    },
    /// `session/new` answered `auth_required`: it needs a sign-in first.
    SignIn,
    /// `initialize` failed.
    Initialize,
    /// `session/new` (or a step after it) failed otherwise.
    SessionNew,
    /// It did not complete within the handshake's time.
    TimedOut {
        /// The limit.
        after: std::time::Duration,
    },
}

/// A refused handshake: why, and what the agent had said by then.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    /// Why.
    pub refusal: Refusal,
    /// What `initialize` said.
    pub introduction: Introduction,
}

/// The ACP version Vessel speaks.
pub const SPEAKS: u16 = 1;

/// The version `version` stands for.
pub(crate) const fn number(version: ProtocolVersion) -> u16 {
    version.as_u16()
}

/// What `initialized` says, within the wire's bounds.
pub(crate) fn of(initialized: &acp::InitializeResponse) -> Introduction {
    let version = initialized
        .agent_info
        .as_ref()
        .and_then(|info| info.version.parse().ok());
    let methods = initialized
        .auth_methods
        .iter()
        // Only `agent` methods: a `terminal` one is never sent, since
        // Vessel does not advertise it (and would not run a program the
        // agent names); the enum is open to methods ACP adds later.
        .filter_map(|method| {
            if let acp::AuthMethod::Agent(method) = method {
                Some(method)
            } else {
                None
            }
        })
        .filter_map(|method| {
            let id: AuthMethodId = method.id.0.parse().ok()?;
            Some(AuthMethod {
                id,
                name: cut(&method.name, AuthMethod::MAX_NAME),
                description: method
                    .description
                    .as_deref()
                    .map(|text| cut(text, AuthMethod::MAX_DESCRIPTION)),
            })
        })
        .take(AuthMethod::MAX)
        .collect();
    Introduction { version, methods }
}

/// `text` without control characters, cut at `max` characters.
fn cut(text: &str, max: usize) -> String {
    text.chars().filter(|c| !c.is_control()).take(max).collect()
}
