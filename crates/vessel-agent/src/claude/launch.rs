//! Claude Code's command for one launch (ADR 0057's launch step): what a
//! session asks for, as the flags the CLI takes it from, checked before
//! anything starts.
//!
//! [Decision Log]
//! - Purpose: Claude Code takes the conversation (new or resumed), its
//!   permission route and mode, the model and its MCP servers only on its
//!   command line, so this is where a session's choices become flags, and
//!   where one the driver cannot honour is refused before the agent runs
//!   (`LaunchRefusal`), never dropped.
//! - Only enumerated flags, ids and paths are added; no value of the
//!   session's goes on the command line (`ps` shows it to every local
//!   user). The MCP servers, whose values may be secrets, go in a file
//!   only the node's user can read, in the node's private directory, by
//!   the harness's own writer (`vessel_harness::claude_mcp_file`): the
//!   same file and the same one argument a terminal session of Claude Code
//!   gets. The Agent SDK passes them as JSON on the command line; Vessel
//!   does not. Nothing is added to the environment.
//! - Every value flag is written `--flag=value`, so no value can be read
//!   as a flag of its own.
//! - A new conversation gets its id here (`--session-id`, a fresh UUID),
//!   so the session is ready with its id before the CLI's first frame,
//!   which Claude Code writes only once a turn starts. A resumed one must
//!   be a UUID: `--resume` also takes a name or a transcript's path, and
//!   an id the agent cannot have made is refused, never read as a path.
//! - Permissions: every tool request is asked of the host over the control
//!   protocol (`--permission-prompt-tool=stdio`). The mode is always passed
//!   (`--permission-mode`), so the node always knows it and a setting in
//!   the person's own Claude folder, which the node cannot see, never picks
//!   it (the coordinator's decision, 2026-11-14; the ACP adapter, unlike
//!   this, leaves it to that setting). The modes and the default are
//!   Paseo's (the owner's "the setup exactly as Paseo's"): `plan`,
//!   `default`, `acceptEdits`, `auto` (the default) and
//!   `bypassPermissions`; `dontAsk`, which the CLI documents, is accepted
//!   but not offered. Under Bedrock or Vertex (`CLAUDE_CODE_USE_BEDROCK`,
//!   `CLAUDE_CODE_USE_VERTEX` true in the agent's environment) `auto` is
//!   not offered, a choice of it is refused before spawning, and the
//!   default is `default`, as Paseo does. One exhaustive match maps them,
//!   and only `bypassPermissions` adds `--allow-dangerously-skip-permissions`.
//! - Questions: `AskUserQuestion` is removed (`--disallowedTools=
//!   AskUserQuestion`) exactly when `AgentSetup::questions` is false, the
//!   node's one decision the ACP path also follows (its adapter offers the
//!   tool only to a client that advertises questions), so the two cannot
//!   drift. With it, the agent's questions are asked of a person
//!   (`asks`).
//! - Fork is slice 4's: refused here (`ForkRefusal::CannotFork`), answered
//!   as the adapters answer an agent that cannot fork.

use std::ffi::OsString;
use std::sync::Arc;

use vessel_harness::WrittenMcpFile;
use vessel_proc::{CommandSpec, EnvValue, Environment};
use vessel_proto::{ChoiceValue, HarnessSessionId, OptionChoice, OptionId, OptionValueId};

use crate::error::{AgentError, LaunchRefusal};
use crate::handshake::{AgentSetup, ForkRefusal};
use crate::options::{KindName, OptionError};

/// The option a session's permission mode is chosen with, as Claude Code's
/// ACP adapter names it.
pub(super) const MODE: &str = "mode";

/// What a launch leaves to its session.
#[derive(Debug)]
pub(crate) struct Launched {
    /// The conversation the session runs: minted here for a new one, the
    /// one it resumes otherwise.
    pub(super) conversation: HarnessSessionId,
    /// Whether the launch resumes `conversation` (`--resume`).
    pub(super) resumed: bool,
    /// The MCP configuration file, removed when the session ends.
    _mcp: Option<WrittenMcpFile>,
}

/// A permission mode Claude Code takes (`--permission-mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    Plan,
    Default,
    AcceptEdits,
    Auto,
    DontAsk,
    Bypass,
}

impl Mode {
    /// Every mode a launch takes.
    pub(super) const ALL: [Self; 6] = [
        Self::Plan,
        Self::Default,
        Self::AcceptEdits,
        Self::Auto,
        Self::DontAsk,
        Self::Bypass,
    ];

    /// The modes offered, Paseo's five in Paseo's order.
    pub(super) const OFFERED: [Self; 5] = [
        Self::Plan,
        Self::Default,
        Self::AcceptEdits,
        Self::Auto,
        Self::Bypass,
    ];

    /// Its name, in the option and on the command line.
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Default => "default",
            Self::AcceptEdits => "acceptEdits",
            Self::Auto => "auto",
            Self::DontAsk => "dontAsk",
            Self::Bypass => "bypassPermissions",
        }
    }

    fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.name() == name)
    }
}

/// `command` (Claude Code's program and its stream-json arguments, as
/// `vessel_harness::launch_direct` built it) shaped for one launch from
/// `setup`.
///
/// # Errors
/// [`AgentError::Refused`]: a fork; a resumed id that is not a UUID; an
/// option other than the mode, or a mode Claude Code does not take; MCP
/// servers without the node's private directory, or their file not
/// written there.
pub(crate) fn launch(
    command: &CommandSpec,
    setup: &AgentSetup,
) -> Result<(CommandSpec, Launched), AgentError> {
    if setup.fork.is_some() {
        return Err(LaunchRefusal::Fork(ForkRefusal::CannotFork).into());
    }
    let auto_unavailable = auto_unavailable_on(command);
    let mode = match mode(&setup.options, auto_unavailable)? {
        Some(chosen) => chosen,
        None if auto_unavailable.is_some() => Mode::Default,
        None => Mode::Auto,
    };
    let mut args: Vec<OsString> = vec![
        "--include-partial-messages".into(),
        "--permission-prompt-tool=stdio".into(),
    ];
    if !setup.questions {
        args.push("--disallowedTools=AskUserQuestion".into());
    }
    let resumed = setup.resume.is_some();
    let conversation = if let Some(resumed) = &setup.resume {
        let Ok(id) = uuid::Uuid::parse_str(resumed.as_str()) else {
            return Err(LaunchRefusal::Conversation(resumed.to_string()).into());
        };
        args.push(flag("--resume=", &id.hyphenated().to_string()));
        HarnessSessionId::new(id.hyphenated().to_string())
    } else {
        let id = uuid::Uuid::new_v4().hyphenated().to_string();
        args.push(flag("--session-id=", &id));
        HarnessSessionId::new(id)
    };
    args.push(flag("--permission-mode=", mode.name()));
    if mode == Mode::Bypass {
        args.push("--allow-dangerously-skip-permissions".into());
    }
    if let Some(model) = &setup.model {
        args.push(flag("--model=", model.as_str()));
    }
    let mcp = mcp_file(setup)?;
    let mcp = match mcp {
        Some((argument, file)) => {
            args.push(argument);
            Some(file)
        }
        None => None,
    };
    let shaped = command.clone().args(args);
    Ok((
        shaped,
        Launched {
            conversation,
            resumed,
            _mcp: mcp,
        },
    ))
}

fn flag(name: &str, value: &str) -> OsString {
    let mut flag = OsString::from(name);
    flag.push(value);
    flag
}

/// The provider `auto` is unavailable on, when the agent's environment
/// sends Claude Code to Bedrock or Vertex (Paseo's
/// `claudeAutoModeUnavailableOn`): the command's own setting, else, when
/// the agent inherits the node's environment, the node's, which it then
/// sees (as `vessel_harness`'s launch checks what the child inherits).
fn auto_unavailable_on(command: &CommandSpec) -> Option<&'static str> {
    let set = |name: &str| {
        let value = match command.env_change(name) {
            Some(EnvValue::Set(value)) => Some(value.clone()),
            Some(EnvValue::Remove) => None,
            None => match command.environment {
                Environment::Inherit => std::env::var_os(name),
                Environment::Clear => None,
            },
        };
        value.is_some_and(|value| {
            let value = value.to_string_lossy().trim().to_ascii_lowercase();
            !matches!(value.as_str(), "" | "0" | "false" | "no" | "off")
        })
    };
    if set("CLAUDE_CODE_USE_BEDROCK") {
        Some("Bedrock")
    } else if set("CLAUDE_CODE_USE_VERTEX") {
        Some("Vertex")
    } else {
        None
    }
}

/// The permission mode chosen, if any; any other option is refused (the
/// options slice adds model, effort and speed), and `auto` where it is
/// unavailable (`auto_unavailable`).
fn mode(
    options: &[OptionChoice],
    auto_unavailable: Option<&'static str>,
) -> Result<Option<Mode>, LaunchRefusal> {
    let offered = |id: &OptionId, value: &OptionValueId| OptionError::NotOffered {
        id: id.clone(),
        value: value.clone(),
        offered: Mode::OFFERED
            .iter()
            .filter(|mode| !(**mode == Mode::Auto && auto_unavailable.is_some()))
            .map(|mode| OptionValueId::new(mode.name()))
            .collect(),
    };
    let mut chosen = None;
    for choice in options {
        if choice.id.as_str() != MODE {
            return Err(OptionError::Unknown {
                id: choice.id.clone(),
                offered: vec![OptionId::new(MODE)],
            }
            .into());
        }
        let ChoiceValue::Select { value } = &choice.value else {
            return Err(OptionError::WrongKind {
                id: choice.id.clone(),
                kind: KindName::Select,
            }
            .into());
        };
        let Some(mode) = Mode::named(value.as_str()) else {
            return Err(offered(&choice.id, value).into());
        };
        if let (Mode::Auto, Some(provider)) = (mode, auto_unavailable) {
            tracing::info!(provider, "auto mode refused: it needs the Anthropic API");
            return Err(offered(&choice.id, value).into());
        }
        // A later choice of the same option wins, as it does over ACP.
        chosen = Some(mode);
    }
    Ok(chosen)
}

/// The MCP servers' file, written in the node's private directory, and
/// the argument naming it.
fn mcp_file(setup: &AgentSetup) -> Result<Option<(OsString, WrittenMcpFile)>, LaunchRefusal> {
    if setup.mcp_servers.is_empty() {
        return Ok(None);
    }
    let dir = setup
        .private_dir
        .as_deref()
        .ok_or(LaunchRefusal::NoPrivateDir)?;
    let refused = |error: vessel_harness::HarnessError| LaunchRefusal::Mcp(Arc::new(error));
    let Some((argument, file)) =
        vessel_harness::claude_mcp_file(&setup.mcp_servers, dir).map_err(refused)?
    else {
        return Ok(None);
    };
    let written = file
        .write()
        .map_err(|error| refused(vessel_harness::HarnessError::McpFile(Arc::new(error))))?;
    Ok(Some((argument, written)))
}

#[cfg(test)]
mod tests;
