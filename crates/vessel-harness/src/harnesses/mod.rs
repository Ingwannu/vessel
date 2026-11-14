//! Per-harness knowledge, one module per [`HarnessKind`].
//!
//! [`descriptor`] is the only `match` on `HarnessKind` that decides
//! behaviour in the whole workspace. Adding a harness variant fails to
//! compile here until its module exists. Where a harness needs logic rather
//! than data (finding a transcript, writing MCP servers), its module has a
//! function and the descriptor points to it.

mod amp;
mod antigravity;
mod claude;
mod codex;
mod cursor;
mod gemini;
mod grok;
mod muse;
mod omp;
mod opencode;
mod pi;

#[cfg(feature = "fakes")]
pub(crate) use claude::slug as claude_slug;

use std::path::{Path, PathBuf};

use vessel_proto::{HarnessKind, HarnessSessionId, ResolvedMcpServers};

use crate::layout::HarnessLayout;
use crate::lookup::Lookup;
use crate::mcp::{McpError, TerminalMcp};
use crate::memory::{MemoryFilesError, RebaseError};
use crate::modes::ModeTag;
use crate::routing::Routing;
use crate::sessions::{SessionFiles, SessionFilesError};

/// What Vessel needs to know to run one harness.
#[derive(Debug)]
pub(crate) struct Descriptor {
    /// Executable name looked up on the search path (without `.exe`).
    pub(crate) binary: &'static str,
    /// The name of the harness's implicit default profile (ADR 0011): its
    /// command, which is what people type.
    pub(crate) profile: &'static str,
    /// How structured (ACP) mode is started.
    pub(crate) structured: Structured,
    /// What the node's environment gives the harness's processes.
    pub(crate) inherits: Inheritance,
    /// Its permission modes as Paseo tiers them (`crate::modes`); empty
    /// when no ids are known yet.
    pub(crate) modes: &'static [ModeTag],
    /// Where the harness keeps its configuration, logins and sessions;
    /// `None` where its vendor does not document it (nothing is guessed:
    /// no memory, no account homes, no fleet files for it).
    pub(crate) home: Option<Home>,
    /// Where shared configuration (instructions, skills) lives in the home.
    pub(crate) layout: HarnessLayout,
    /// Arguments after the binary that report whether it is logged in,
    /// without changing anything; `None` if the harness has no such command.
    pub(crate) login: Option<&'static [&'static str]>,
    /// How to find one session's files under the home; `None` if they
    /// cannot be moved between homes.
    pub(crate) sessions: Option<FindSession>,
    /// Where a profile's memory is under the home (one project's, or
    /// all); `None` if Vessel does not know.
    pub(crate) memory: Option<FindMemory>,
    /// How a home-relative path is named for another working directory;
    /// `None` when paths do not depend on it.
    pub(crate) rebase: Option<Rebase>,
    /// How a terminal session is given MCP servers at launch; `None` if the
    /// harness has no launch-time form (structured sessions still get them
    /// over ACP).
    pub(crate) terminal_mcp: Option<RenderMcp>,
    /// The vendor's documented install (`vessel init` and `agent.install`
    /// offer it, and run it only when the person says yes and it is a
    /// package manager's command).
    pub(crate) install: Installer,
    /// Arguments after the binary that log a person in; printed for them
    /// to run, never run by Vessel. `None` when logging in happens inside
    /// the harness's own interface.
    pub(crate) login_command: Option<&'static [&'static str]>,
    /// Files whose presence in a directory beside the default home (`.claude`
    /// next to `.claude-<name>`) marks it as another account's home; only
    /// names are looked at. Empty: no per-account homes to find.
    pub(crate) account_files: &'static [&'static str],
    /// Whether a profile may add arguments to its terminal command line.
    /// Every harness so far takes them; a harness whose terminal command
    /// must stay fixed would say `false`, and its profiles with `args` are
    /// refused when `profiles.toml` is read.
    pub(crate) terminal_args: bool,
    /// How the harness is pointed at the node's model proxy (ADR 0015);
    /// `None` if it cannot be yet, and its profiles with a `model` route
    /// are refused when `profiles.toml` is read.
    pub(crate) model_proxy: Option<Routing>,
}

/// Find a session's files: `(home, working directory, session id)`, the
/// id already checked to be one plain file name.
pub(crate) type FindSession =
    fn(&Path, &Path, &HarnessSessionId) -> Result<Option<SessionFiles>, SessionFilesError>;

/// Find a profile's memory directories, relative to the home `(home,
/// project)`: one project's, by its working directory, or every one.
pub(crate) type FindMemory = fn(&Path, Option<&Path>) -> Result<Vec<PathBuf>, MemoryFilesError>;

/// Rename a home-relative path `(path, from, to)` for working directory
/// `to` instead of `from`.
pub(crate) type Rebase = fn(&Path, &Path, &Path) -> Result<PathBuf, RebaseError>;

/// Render MCP servers into the harness's launch-time form.
pub(crate) type RenderMcp = fn(&ResolvedMcpServers) -> Result<TerminalMcp, McpError>;

/// How structured mode starts.
#[derive(Debug)]
pub(crate) enum Structured {
    /// The harness binary itself speaks ACP when given these arguments.
    Native {
        /// Arguments that switch the harness into ACP mode.
        args: &'static [&'static str],
        /// Variables its vendor asks for in that mode (Muse's
        /// `MUSE_EXPERIMENTAL_ACP=on`), set before the profile's own.
        env: &'static [(&'static str, &'static str)],
    },
    /// A separate adapter program speaks ACP and drives the harness.
    Adapter {
        /// The adapter's executable name.
        binary: &'static str,
        /// How a user installs it; shown when it is missing.
        install: Installer,
        /// Whether a third party, not the agent's vendor, makes it: said
        /// wherever the adapter is offered (Amp's `amp-acp`).
        third_party: bool,
    },
    /// No ACP of the harness's own: only a direct driver (ADR 0057) runs
    /// it, and until that driver is built and chosen a structured session
    /// is refused, naming what it needs (ADR 0058: Pi, OMP, Antigravity).
    None {
        /// The protocol it needs, as a person reads it.
        needs: &'static str,
    },
}

impl Structured {
    /// Why a structured session of the harness cannot start where `lookup`
    /// finds programs (the harness itself found); `None` when it can.
    pub(crate) fn unavailable(&self, lookup: &Lookup) -> Option<String> {
        match self {
            Self::Native { .. } => None,
            Self::Adapter {
                binary,
                install,
                third_party,
            } => lookup.find(binary).is_none().then(|| {
                let whose = if *third_party {
                    ", which a third party makes,"
                } else {
                    ""
                };
                let how = match *install {
                    Installer::Run(command) => format!("install it with `{command}`"),
                    Installer::ByHand(step) => step.to_owned(),
                };
                format!("its ACP adapter `{binary}`{whose} is not installed ({how})")
            }),
            Self::None { needs } => Some(format!(
                "it has no ACP; it runs over {needs}, which this node does not drive yet"
            )),
        }
    }
}

/// What the node's environment gives a harness's processes.
///
/// [Decision Log]
/// - Purpose: the four inherit the node's environment, as when a person
///   runs them in a shell (their vendors are the ones the person chose when
///   installing Vessel's first agents). ADR 0058 gives the seven ADR 0032's
///   clean environment instead: they run the person's tools with whatever
///   keys the node holds, and some (Amp's adapter) come from third parties.
/// - Choice: `Clean` starts empty, adds what an added agent gets
///   (`crate::added::clean`: the search path, the home, the system's own
///   variables) and then the vendor's documented variables by name
///   (`XAI_API_KEY`), read from the node's environment; the home variable
///   and the profile's own changes come after, as for the four.
/// - Cost: a variable a vendor reads but does not document is missing
///   until it is listed here; a profile can set it (`profiles.toml`).
#[derive(Debug)]
pub(crate) enum Inheritance {
    /// The node's whole environment (the four).
    Node,
    /// The clean environment plus these variables, when the node has them.
    Clean {
        /// The vendor's documented variables.
        names: &'static [&'static str],
    },
}

/// A vendor's documented install.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Installer {
    /// A package manager's command (`npm install -g ...`), which Vessel
    /// runs when a person says yes.
    Run(&'static str),
    /// A command or step a person runs themselves (a script piped to a
    /// shell, a download): Vessel shows it and never runs it.
    ByHand(&'static str),
}

impl Installer {
    /// The command or step, as a person reads it.
    pub(crate) const fn text(self) -> &'static str {
        match self {
            Self::Run(text) | Self::ByHand(text) => text,
        }
    }

    /// Whether Vessel may run it.
    pub(crate) const fn runs(self) -> bool {
        matches!(self, Self::Run(_))
    }
}

/// A harness's home directory.
#[derive(Debug)]
pub(crate) struct Home {
    /// The environment variable that moves it, if the harness documents one.
    pub(crate) variable: Option<&'static str>,
    /// Where it is by default, relative to the user's home directory.
    pub(crate) default: &'static [&'static str],
}

/// The descriptor of `kind`.
pub(crate) const fn descriptor(kind: HarnessKind) -> &'static Descriptor {
    match kind {
        HarnessKind::ClaudeCode => &claude::DESCRIPTOR,
        HarnessKind::Codex => &codex::DESCRIPTOR,
        HarnessKind::Gemini => &gemini::DESCRIPTOR,
        HarnessKind::OpenCode => &opencode::DESCRIPTOR,
        HarnessKind::Grok => &grok::DESCRIPTOR,
        HarnessKind::Pi => &pi::DESCRIPTOR,
        HarnessKind::Omp => &omp::DESCRIPTOR,
        HarnessKind::Antigravity => &antigravity::DESCRIPTOR,
        HarnessKind::Muse => &muse::DESCRIPTOR,
        HarnessKind::Amp => &amp::DESCRIPTOR,
        HarnessKind::Cursor => &cursor::DESCRIPTOR,
    }
}
