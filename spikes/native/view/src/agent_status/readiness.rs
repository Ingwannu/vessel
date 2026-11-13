//! Facts the node actually checked, kept separate from discovery. A
//! successful ACP handshake is not proof of a working sign-in (ADR 0032).
//!
//! [Decision Log]
//! - Purpose: explain installation, conversation support and sign-in
//!   without treating a missing check as success.
//! - Constraints: discovery has no authentication verdict; added programs
//!   must never be run with guessed version or login arguments.
//! - Alternatives: reuse the discovery headline for all three facts.
//! - Choice: render the node's exhaustive readiness enums separately and
//!   offer only the install parts that discovery says are missing.
//! - Why: a terminal agent can be installed with its ACP adapter missing.
//! - Cost: the discovery headline remains for callers that have no status.

use vessel_proto::agents::{
    AgentDetail, AgentInfo, InstallOffer, InstallPart, Installed, SignedIn,
};
use vessel_proto::{HarnessRef, StructuredSupport};

use crate::Lang;

/// Installation and conversation availability, in the person's language.
#[must_use]
pub fn installation(lang: Lang, installed: &Installed) -> String {
    let words = lang.agents();
    match installed {
        Installed::Found {
            structured: StructuredSupport::Available,
            ..
        } => words.ready.to_owned(),
        Installed::Found {
            structured: StructuredSupport::Unavailable { reason },
            ..
        } => words.no_conversation(reason),
        Installed::Missing { program } => words.missing(program.as_str()),
        Installed::TooOld { speaks, needs, .. } | Installed::TooNew { speaks, needs, .. } => {
            words.incompatible(*speaks, *needs)
        }
        Installed::Unknown { reason } => words.install_unknown(reason),
    }
}

/// Authentication is reported only as far as the machine knows it.
#[must_use]
pub fn authentication(lang: Lang, signed_in: &SignedIn) -> String {
    let words = lang.agents();
    match signed_in {
        SignedIn::Yes => words.credentials.to_owned(),
        SignedIn::No { .. } => words.signed_out.to_owned(),
        SignedIn::NotRequired => words.not_required.to_owned(),
        SignedIn::Unknown { reason } => words.auth_unknown(reason),
    }
}

/// Version as reported, including an incompatible agent's version.
#[must_use]
pub fn version(installed: &Installed) -> Option<&str> {
    match installed {
        Installed::Found { version, .. }
        | Installed::TooOld { version, .. }
        | Installed::TooNew { version, .. } => version
            .as_ref()
            .map(vessel_proto::agents::AgentVersion::as_str),
        Installed::Missing { .. } | Installed::Unknown { .. } => None,
    }
}

/// Offered installers relevant to the missing part; unknown checks do
/// not justify an install. Added agents have no vendor installer.
#[must_use]
pub fn installs<'a>(
    agent: &HarnessRef,
    installed: &Installed,
    agents: &'a [AgentInfo],
) -> Vec<&'a InstallOffer> {
    let part = match installed {
        Installed::Missing { .. } => InstallPart::Harness,
        Installed::Found {
            structured: StructuredSupport::Unavailable { .. },
            ..
        } => InstallPart::Adapter,
        Installed::Found {
            structured: StructuredSupport::Available,
            ..
        }
        | Installed::TooOld { .. }
        | Installed::TooNew { .. }
        | Installed::Unknown { .. } => return Vec::new(),
    };
    agents
        .iter()
        .filter(|info| info.agent == *agent)
        .flat_map(|info| match &info.detail {
            AgentDetail::Known { installs, .. } => {
                installs.iter().filter(|offer| offer.part == part).collect()
            }
            AgentDetail::Added { .. } => Vec::new(),
        })
        .collect()
}
