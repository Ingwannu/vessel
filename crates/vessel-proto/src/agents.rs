//! Agents beyond the ones Vessel knows (protocol 11, ADR 0032): how one is
//! named on the wire, what a node keeps of the ones its owner added, and
//! what it knows of each agent's installation and sign-in. Protocol 21
//! (ADR 0058) made seven more agents known; this module's withholding
//! (`older`) covers them too.
//!
//! [Decision Log]
//! - Purpose: a node runs any agent that speaks ACP, not only Claude Code,
//!   Codex, Gemini CLI and `OpenCode`, without breaking a peer that knows only
//!   those four (protocols 2 to 10).
//! - Constraint: [`HarnessKind`](crate::HarnessKind) is matched
//!   exhaustively wherever Vessel knows something about an agent (its home,
//!   its login, its session files, fleet sync, routing). That knowledge
//!   does not exist for an agent a person added by its command, and must
//!   not be guessed.
//! - Alternatives: a fifth `HarnessKind` variant carrying a name (every
//!   `match` on the four would gain an arm that pretends to know, and the
//!   type would lose `Copy`); a free string for every agent (the four would
//!   lose their exhaustive matches); added names in the same space of
//!   strings as the four (a stored session would change meaning the day
//!   Vessel knows a fifth agent by a name someone had used).
//! - Choice: [`HarnessRef`], the four as they are or an added agent by its
//!   [`AgentName`]; on the wire the four stay the strings they were and an
//!   added one is `{"added": "<name>"}`. Code that needs knowledge matches
//!   it and says, for `Added`, that the feature is not offered.
//! - Cost: every place that took a `HarnessKind` from a spec decides what an
//!   added agent gets there; the compiler lists them.

mod added;
mod calls;
mod catalogue;
mod install;
mod name;
mod older;
mod status;
mod text;

pub use added::{
    AddedAgent, AgentDefinition, AgentDetail, AgentInfo, AgentSource, AgentToAdd, AgentVariable,
    AuthMethod, InstallOffer, InstallPart, SecretEnv, SignInHelp,
};
pub use calls::{AddAgent, ListAgents, RemoveAgent};
pub use catalogue::{Catalogue, CatalogueEntry, CatalogueRun, CatalogueVariable, GetCatalogue};
pub use install::{InstallAgent, InstallJob, InstallOutput, InstallState, ReadInstall};
pub use name::{AgentName, AgentNameError, HarnessRef, TerminalSupport, UnknownAgent};
pub use older::{
    READS_ADDED_AGENTS, READS_EVERY_AGENT, READS_TEN_AGENTS, readable, reads, unreadable,
};
pub(crate) use older::{
    agents as agents_for, catalogue as catalogue_for, harnesses as harnesses_for,
    profiles as profiles_for, prompts as prompts_for, readiness as readiness_for,
    schedules as schedules_for, usage as usage_for,
};
pub use status::{AgentReadiness, CheckStep, GetAgentStatus, Installed, SignedIn, UnknownReason};
pub use text::{
    AgentLabel, AgentProgram, AgentVersion, AuthMethodId, InvalidAgentText, RegistryId,
    RegistryVersion, Sha256Hex,
};
