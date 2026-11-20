//! Terminal agent hooks (ADR 0089, "The CLI consumer: agent hooks"): an
//! agent running in a Vessel terminal reports its own activity, through
//! `vessel hooks <agent> <event>`, which its hook configuration runs.
//!
//! [Decision Log]
//! - Purpose: a terminal's attention (finished, needs input) is only as good
//!   as the activity reported for it. A shell cannot tell when the agent in
//!   it starts or stops a turn, but the agent's own hooks can.
//! - Reference: Paseo 0.15.4, `cli/dist/commands/hooks.js` and
//!   `server/dist/server/terminal/agent-hooks/` (`provider-registry.js`,
//!   `claude/`, `codex/`, `opencode/`, `agent-hook-installer.js`,
//!   `terminal-agent-hook-setting.js`). The event maps below are Paseo's,
//!   event for event.
//! - Existing assets: the activity-only capability every terminal gets
//!   (`VESSEL_TERMINAL_ACTIVITY_*`, `terminals/producer.rs`), the local IPC
//!   endpoint that admits it, and the agents' homes as the node starts them
//!   (`HarnessEnv::home`).
//! - Choice:
//!   - This module holds the maps and the installer. The CLI reads the maps,
//!     and the node's setting runs the installer.
//!   - Where to install is each agent's home as Vessel runs it: Claude Code
//!     `$CLAUDE_CONFIG_DIR` or `~/.claude`, Codex `$CODEX_HOME` or
//!     `~/.codex`, `OpenCode` `~/.config/opencode`. Paseo also honours
//!     `OPENCODE_CONFIG_DIR` and `XDG_CONFIG_HOME` for `OpenCode`. Vessel
//!     does not set those for the `OpenCode` it runs, so the hooks go where
//!     that `OpenCode` reads.
//!   - Vessel's entries are recognised by their exact command: one of the
//!     forms [`shell_command`] and [`windows_command`] write for one of the
//!     agent's events, and nothing looser ([`marked`]). A person's command
//!     that only mentions `vessel hooks` is theirs. If a form ever changes,
//!     `marked` must keep recognising the old one, or turning the setting off
//!     would leave the old entries behind.
//!   - The commands quote the agent and the event (`hooks "claude" "Stop"`).
//!     Paseo runs beside Vessel on the same machines and writes the same
//!     files. Its install and uninstall drop every hook command containing
//!     its marker, `hooks claude` or `hooks codex` (Paseo 0.15.4
//!     `claude/claude.ts:25`, `codex/codex.ts:26`, and the removal in
//!     `claude/claude-settings.ts:98-113`, `codex/codex-settings.ts:105-139`).
//!     Unquoted, Vessel's commands contain it, and every Paseo restart would
//!     remove Vessel's hooks. Paseo's own installer was run over Vessel's
//!     entries to check this (`tests/fixtures/agent_hooks/generate.mjs`); the
//!     unquoted form fails there.
//! - Rules beyond Paseo (the coordinator, 2026-11-20), because these files
//!   drive a person's agents:
//!   - a one-time backup before the first write to an existing file;
//!   - only marked entries change;
//!   - an invalid file is left untouched, with one logged line;
//!   - turning the setting off removes exactly the marked entries;
//!   - tests write only under a scratch home, and a guard refuses the real
//!     one.

use std::path::PathBuf;

use vessel_harness::{HarnessEnv, Profile};
use vessel_proto::HarnessKind;
use vessel_proto::notifications::TerminalActivitySignal;

mod config_file;
mod files;
mod plugin_file;

/// An agent whose hooks Vessel installs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agent {
    /// Claude Code: `settings.json` hooks.
    Claude,
    /// Codex: `hooks.json`.
    Codex,
    /// `OpenCode`: a plugin file.
    OpenCode,
}

impl Agent {
    /// Every agent, in Paseo's order.
    pub const ALL: [Self; 3] = [Self::Claude, Self::Codex, Self::OpenCode];

    /// The name `vessel hooks` takes (Paseo's provider id).
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::OpenCode => "opencode",
        }
    }

    /// The agent `vessel hooks` names, matched without case as Paseo does.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|agent| agent.id().eq_ignore_ascii_case(name))
    }

    /// The events its hooks report, each with its signal (Paseo's maps:
    /// `claude/claude.js:2-7`, `codex/codex.js:2-8`, `opencode/opencode.js:2-8`).
    /// Claude Code's `Notification` is decided by its input
    /// ([`Event::signal`]).
    const fn events(self) -> &'static [(&'static str, Option<TerminalActivitySignal>)] {
        use TerminalActivitySignal::{Idle, NeedsInput, Working};
        match self {
            Self::Claude => &[
                ("UserPromptSubmit", Some(Working)),
                ("Stop", Some(Idle)),
                ("StopFailure", Some(Idle)),
                ("SessionEnd", Some(Idle)),
                ("Notification", None),
            ],
            Self::Codex => &[
                ("UserPromptSubmit", Some(Working)),
                ("PreToolUse", Some(Working)),
                ("PostToolUse", Some(Working)),
                ("PermissionRequest", Some(NeedsInput)),
                ("Stop", Some(Idle)),
            ],
            Self::OpenCode => &[
                ("session.status.busy", Some(Working)),
                ("session.status.retry", Some(Working)),
                ("session.status.idle", Some(Idle)),
                ("permission.asked", Some(NeedsInput)),
                ("permission.replied", Some(Working)),
            ],
        }
    }

    /// Its hook event names, for the installer.
    fn event_names(self) -> impl Iterator<Item = &'static str> {
        self.events().iter().map(|(name, _signal)| *name)
    }

    /// The harness whose home its configuration lives in.
    const fn harness(self) -> HarnessKind {
        match self {
            Self::Claude => HarnessKind::ClaudeCode,
            Self::Codex => HarnessKind::Codex,
            Self::OpenCode => HarnessKind::OpenCode,
        }
    }
}

/// One hook event an agent reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    agent: Agent,
    /// `None`: decided by the hook's input.
    signal: Option<TerminalActivitySignal>,
}

impl Event {
    /// The event `vessel hooks <agent> <event>` names; `None` for an agent
    /// or event Vessel does not map, which reports nothing.
    #[must_use]
    pub fn parse(agent: &str, event: &str) -> Option<Self> {
        let agent = Agent::parse(agent)?;
        let (_name, signal) = agent
            .events()
            .iter()
            .find(|(name, _signal)| *name == event)?;
        Some(Self {
            agent,
            signal: *signal,
        })
    }

    /// Whether its signal depends on the hook's standard input (Claude
    /// Code's `Notification`). Only then is the input read.
    #[must_use]
    pub const fn reads_input(&self) -> bool {
        self.signal.is_none()
    }

    /// The activity to report, given the hook's input when
    /// [`reads_input`](Self::reads_input). Claude Code's `Notification`
    /// needs input only for its idle prompt (`claude/claude.js:39-42`);
    /// any other notification, or no input, reports nothing.
    #[must_use]
    pub fn signal(&self, input: Option<&str>) -> Option<TerminalActivitySignal> {
        if let Some(signal) = self.signal {
            return Some(signal);
        }
        let notification: serde_json::Value = serde_json::from_str(input?).ok()?;
        (notification.get("notification_type")?.as_str()? == "idle_prompt")
            .then_some(TerminalActivitySignal::NeedsInput)
    }

    /// The agent that reported it.
    #[must_use]
    pub const fn agent(&self) -> Agent {
        self.agent
    }
}

/// Where each agent keeps the configuration its hooks go in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Targets {
    claude: Option<PathBuf>,
    codex: Option<PathBuf>,
    opencode: Option<PathBuf>,
}

impl Targets {
    /// Each agent's home as the node starts it.
    pub(crate) fn of(env: &HarnessEnv) -> Self {
        let home = |agent: Agent| env.home(&Profile::default_for(agent.harness()));
        Self {
            claude: home(Agent::Claude),
            codex: home(Agent::Codex),
            opencode: home(Agent::OpenCode),
        }
    }

    fn home(&self, agent: Agent) -> Option<&PathBuf> {
        match agent {
            Agent::Claude => self.claude.as_ref(),
            Agent::Codex => self.codex.as_ref(),
            Agent::OpenCode => self.opencode.as_ref(),
        }
    }

    /// Install (`on`) or remove every agent's hooks; blocking file work, so
    /// call it off the async threads. Each agent's outcome is logged, and
    /// one failing does not stop the others.
    pub(crate) fn apply(&self, on: bool) {
        #[cfg(test)]
        files::refuse_real_home(self);
        for agent in Agent::ALL {
            let Some(home) = self.home(agent) else {
                continue;
            };
            let outcome = match agent {
                Agent::Claude | Agent::Codex => config_file::apply(agent, home, on),
                Agent::OpenCode => plugin_file::apply(home, on),
            };
            files::log(agent, on, &outcome);
        }
    }
}

/// The command a hook runs (Paseo's `agent-hook-installer.js:43-46`): only
/// inside a Vessel terminal, with `VESSEL_HOOK_CLI` naming another `vessel`.
/// The agent and event are quoted so the command never contains Paseo's
/// marker (see the Decision Log).
fn shell_command(agent: Agent, event: &str) -> String {
    format!(
        "if [ -n \"${}\" ]; then \"${{{}:-vessel}}\" hooks \"{}\" \"{}\"; fi",
        vessel_proto::notifications::TERMINAL_ACTIVITY_ID_ENV,
        HOOK_CLI_ENV,
        agent.id(),
        event
    )
}

/// The same for `cmd.exe` (Paseo's `agent-hook-installer.js:47-50`).
fn windows_command(agent: Agent, event: &str) -> String {
    let id = vessel_proto::notifications::TERMINAL_ACTIVITY_ID_ENV;
    let args = format!("hooks \"{}\" \"{}\"", agent.id(), event);
    format!(
        "if defined {id} (if defined {HOOK_CLI_ENV} (\"%{HOOK_CLI_ENV}%\" {args}) else (vessel {args})) else (exit /b 0)"
    )
}

/// What names another `vessel` for the hooks (Paseo's `PASEO_HOOK_CLI`).
const HOOK_CLI_ENV: &str = "VESSEL_HOOK_CLI";

/// Whether `command` is exactly one Vessel installs for one of `agent`'s
/// events.
fn marked(command: &str, agent: Agent) -> bool {
    agent.event_names().any(|event| {
        command == shell_command(agent, event) || command == windows_command(agent, event)
    })
}

#[cfg(test)]
mod tests;
