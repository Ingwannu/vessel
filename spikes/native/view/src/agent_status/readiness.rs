//! Facts the node checked, distinct from discovery and from sign-in.
//!
//! [Decision Log]
//! - Purpose: a short state in the list, a complete instruction beneath
//!   it, and the node's diagnostic prose separately labelled in details.
//! - Constraints: unknown sign-in is not success; an executable can be
//!   installed while its chat adapter is missing; login is on the node.
//! - Alternatives: combine the diagnostic with the translated sentence,
//!   or use a successful connection as proof of authentication.
//! - Choice: typed badge values, separate installation/authentication
//!   sentences, labelled machine messages and typed sign-in steps.
//! - Why: people can act without knowing protocol vocabulary, while the
//!   original diagnostic and account information stay available.
//! - Cost: the machine's diagnostic itself remains in its own language.

use vessel_proto::agents::{
    AgentDetail, AgentInfo, AgentVersion, AuthMethod, InstallOffer, InstallPart, Installed,
    SignedIn, UnknownReason,
};
use vessel_proto::{EnvName, HarnessRef, StructuredSupport};

use crate::Lang;

use crate::badge::Variant;

/// Short installation/chat state. Authentication stays a separate fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentBadge {
    /// Installed with chat support; says nothing about sign-in.
    Available,
    /// Its program was not found.
    NotInstalled,
    /// Installed but needs an adapter or a compatible version.
    NeedsAttention,
    /// A check failed or timed out.
    Error,
    /// The machine has no installation verdict.
    Unknown,
}

impl AgentBadge {
    /// Derive the row's short state from the same installation facts as its explanation.
    #[must_use]
    pub const fn of(installed: &Installed) -> Self {
        match installed {
            Installed::Found {
                structured: StructuredSupport::Available,
                ..
            } => Self::Available,
            Installed::Found {
                structured: StructuredSupport::Unavailable { .. },
                ..
            }
            | Installed::TooOld { .. }
            | Installed::TooNew { .. } => Self::NeedsAttention,
            Installed::Missing { .. } => Self::NotInstalled,
            Installed::Unknown {
                reason: UnknownReason::Failed { .. } | UnknownReason::TimedOut { .. },
            } => Self::Error,
            Installed::Unknown {
                reason:
                    UnknownReason::NotInstalled
                    | UnknownReason::NotChecked
                    | UnknownReason::InUse
                    | UnknownReason::NotSaid
                    | UnknownReason::NoHome
                    | UnknownReason::Unread,
                ..
            } => Self::Unknown,
        }
    }

    /// The status badge's semantic variant, independent of any toolkit.
    #[must_use]
    pub const fn variant(self) -> Variant {
        match self {
            Self::Available => Variant::Success,
            Self::NotInstalled | Self::Unknown => Variant::Muted,
            Self::NeedsAttention => Variant::Warning,
            Self::Error => Variant::Error,
        }
    }
}

/// The label and semantic variant a screen passes to the kit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Badge {
    /// Short, translated state after Paseo's provider rows.
    pub label: &'static str,
    /// How the kit presents this kind of state.
    pub variant: Variant,
}

/// Diagnostic prose is explicitly attributed, never spliced into a translation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MachineMessage<'a> {
    /// A translated attribution label.
    pub label: &'static str,
    /// The node's exact message, in the language it supplied.
    pub text: &'a str,
}

impl MachineMessage<'_> {
    /// Diagnostic prose is informational; it is not itself a readiness verdict.
    #[must_use]
    pub const fn variant(self) -> crate::alert::Variant {
        crate::alert::Variant::Info
    }
}

/// The row's state and its complete explanation, with optional diagnostic detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installation<'a> {
    /// Short status for the list row.
    pub badge: Badge,
    /// A complete sentence that does not depend on the diagnostic's language.
    pub sentence: String,
    /// The original diagnostic, drawn beneath and marked as the machine's.
    pub machine_message: Option<MachineMessage<'a>>,
}

/// Installation and chat support in the person's language. An adapter
/// instruction appears only when the node offers that exact action.
#[must_use]
pub fn installation<'a>(
    lang: Lang,
    agent: &HarnessRef,
    installed: &'a Installed,
    agents: &[AgentInfo],
) -> Installation<'a> {
    let words = lang.agents();
    let state = AgentBadge::of(installed);
    let (sentence, machine_message) = match installed {
        Installed::Found {
            structured: StructuredSupport::Available,
            ..
        } => (words.ready.to_owned(), None),
        Installed::Found {
            structured: StructuredSupport::Unavailable { reason },
            ..
        } => {
            let machine_message = Some(MachineMessage {
                label: words.machine_message,
                text: reason,
            });
            let sentence = if installs(agent, installed, agents).is_empty() {
                words.cannot_chat.to_owned()
            } else {
                words.adapter_missing()
            };
            (sentence, machine_message)
        }
        Installed::Missing { .. } => {
            let sentence = if installs(agent, installed, agents).is_empty() {
                words.not_installed.to_owned()
            } else {
                words.program_missing()
            };
            (sentence, None)
        }
        Installed::TooOld { .. } => (words.older_agent(), None),
        Installed::TooNew { .. } => (words.too_new.to_owned(), None),
        Installed::Unknown { reason, .. } => (words.install_unknown(reason), None),
    };
    Installation {
        badge: Badge {
            label: words.badge(state),
            variant: state.variant(),
        },
        sentence,
        machine_message,
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
        SignedIn::Unknown { reason, .. } => words.auth_unknown(reason),
    }
}

/// Only a compatible installed chat program can answer a defaults probe.
/// Unknown authentication is not guessed here; the probe can ask for sign-in.
#[must_use]
pub const fn offers_defaults(installed: &Installed) -> bool {
    matches!(
        installed,
        Installed::Found {
            structured: StructuredSupport::Available,
            ..
        }
    )
}

/// A sign-in step retains the typed facts needed to draw its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignInStep<'a> {
    /// The account selector, by name only; its value never leaves the node.
    AccountFolder(&'a EnvName),
    /// Draw fixed-width with the kit's copy button; never run on the client.
    Command(&'a str),
    /// The agent's own method, as a labelled machine message.
    Method(&'a AuthMethod),
    /// No command or method was supplied: use the agent's interface.
    OpenAgent,
}

/// One labelled, actionable step. A screen renders it, without guessing login commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignInRow<'a> {
    /// A translated step label.
    pub label: &'static str,
    /// The instruction, complete without a raw machine value.
    pub instruction: &'static str,
    /// Its command, account selector or agent-supplied method.
    pub step: SignInStep<'a>,
}

/// How the node says to sign in. A signed-out agent always has at least
/// one step, and no help is advertised for other authentication states.
#[must_use]
pub fn sign_in_help(lang: Lang, signed_in: &SignedIn) -> Vec<SignInRow<'_>> {
    let SignedIn::No { help, .. } = signed_in else {
        return Vec::new();
    };
    let words = lang.agents();
    let mut rows = Vec::new();
    if let Some(home) = &help.home {
        rows.push(SignInRow {
            label: words.account_folder,
            instruction: words.account_instruction,
            step: SignInStep::AccountFolder(home),
        });
    }
    if let Some(command) = &help.command {
        rows.push(SignInRow {
            label: words.sign_in_command,
            instruction: words.command_instruction,
            step: SignInStep::Command(command),
        });
    }
    for method in &help.methods {
        rows.push(SignInRow {
            label: words.sign_in_method,
            instruction: words.method_instruction,
            step: SignInStep::Method(method),
        });
    }
    if help.command.is_none() && help.methods.is_empty() {
        rows.push(SignInRow {
            label: words.open_agent,
            instruction: words.open_instruction,
            step: SignInStep::OpenAgent,
        });
    }
    rows
}

/// Exact path or requested program for a fixed-width copyable value.
#[must_use]
pub fn program(installed: &Installed) -> Option<std::borrow::Cow<'_, str>> {
    match installed {
        Installed::Found { path, .. }
        | Installed::TooOld { path, .. }
        | Installed::TooNew { path, .. } => Some(path.to_string_lossy()),
        Installed::Missing { program, .. } => Some(std::borrow::Cow::Borrowed(program.as_str())),
        Installed::Unknown { .. } => None,
    }
}

/// Version as reported, including an incompatible agent's version.
#[must_use]
pub fn version(installed: &Installed) -> Option<&str> {
    match installed {
        Installed::Found { version, .. }
        | Installed::TooOld { version, .. }
        | Installed::TooNew { version, .. } => version.as_ref().map(AgentVersion::as_str),
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
