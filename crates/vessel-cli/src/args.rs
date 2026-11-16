//! The command line, parsed once into typed values.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use vessel_client::BearerToken;
use vessel_proto::{EventSeq, HarnessRef, ProfileName, SessionTitle};

use crate::selector::SessionSelector;

mod agents;
mod automation;
mod control;
mod conversation;
mod fleet;
mod forge;
mod history;
mod inbox;
mod log;
mod node;
mod nodes;
pub(crate) mod parity;
mod search;
mod terminals;
mod usage;
mod workspace;

pub(crate) use agents::{AddAgentArgs, AgentsCommand};
pub(crate) use automation::{PromptCommand, ScheduleArgs, ScheduleCommand};
pub(crate) use control::{Confirmation, Stop, StopPolicy, StopTimeout};
pub(crate) use conversation::{Compare, ConversationCommand, QueueCommand, Send};
pub(crate) use fleet::{FleetCommand, MemoryCommand, SecretCommand};
pub(crate) use forge::{Method, PrCommand, Switch};
pub(crate) use history::History;
pub(crate) use inbox::InboxCommand;
pub(crate) use log::{Log, LogMode};
pub(crate) use node::{DevicesCommand, Install, NodeCommand, Source, Update};
pub(crate) use nodes::{FromStdin, NodesCommand};
pub(crate) use search::Search;
pub(crate) use terminals::{OpenShell, Ports, ScriptCommand, ScriptPlaceArgs, Shell, ShellCommand};
pub(crate) use usage::{LookupChoice, Period, UsageCommand};
pub(crate) use workspace::{ProjectCommand, WorktreeCommand};

/// Vessel: drive agent harnesses on this and other machines.
#[derive(Debug, Parser)]
#[command(name = "vessel", version, about)]
pub(crate) struct Cli {
    #[command(flatten)]
    pub(crate) output: parity::OutputOptions,
    #[command(flatten)]
    pub(crate) target: Target,
    #[command(subcommand)]
    pub(crate) command: Command,
}

/// Which node to talk to, and with what credentials.
#[derive(Debug, Args)]
pub(crate) struct Target {
    /// Node to talk to: a name from your node book (`vessel nodes`), an
    /// SSH node (`ssh://[user@]host[:port]` or `user@host`), or a websocket
    /// node (`host`, `host:port`, a `ws://` / `http://` URL). Without it:
    /// the project's or the book's `default_node`, else the node on this
    /// machine (127.0.0.1:7077).
    #[arg(long, global = true, env = "VESSEL_NODE")]
    pub(crate) node: Option<String>,
    /// A websocket node's access token. For a node on this machine (a
    /// loopback address) it defaults to the token file of the local data
    /// directory; a book entry may name a token file; any other node needs
    /// it given explicitly. SSH nodes need none.
    #[arg(long, global = true, env = "VESSEL_TOKEN", hide_env_values = true)]
    pub(crate) token: Option<BearerToken>,
    /// Local data directory (default `~/.vessel`): where the token is read
    /// from, and where `node start` keeps its state.
    #[arg(long, global = true, env = "VESSEL_DATA_DIR")]
    pub(crate) data_dir: Option<PathBuf>,
    /// Configuration directory holding `vessel/nodes.toml` (default: the
    /// platform's, e.g. `~/.config`).
    #[arg(long, global = true, env = "VESSEL_CONFIG_DIR", hide = true)]
    pub(crate) config_dir: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Inspect pending permission requests and questions across sessions.
    Permit {
        #[command(subcommand)]
        command: crate::permit::Command,
    },
    /// List and answer attention across sessions on the selected node.
    Inbox {
        #[command(subcommand)]
        command: InboxCommand,
    },
    /// Run a node in the foreground, or install one on another machine.
    Node {
        #[command(subcommand)]
        command: NodeCommand,
    },
    /// Your node book: named nodes for `--node`.
    Nodes {
        #[command(subcommand)]
        command: NodesCommand,
    },
    /// Show the node and the harnesses it can run.
    Status,
    /// List the harnesses installed on the node.
    Harnesses,
    /// List the node's harness profiles (`--profile` for `run`).
    Profiles {
        /// Only this harness's profiles.
        #[arg(long)]
        harness: Option<HarnessRef>,
        /// Have the node read its `profiles.toml` again first (needs the
        /// `admin` scope). A file that does not parse changes nothing.
        #[arg(long)]
        reload: bool,
    },
    /// The agents the node can run, and those added on it by their command
    /// (any agent that speaks ACP, ADR 0032). Lists them without a command.
    Agents {
        #[command(subcommand)]
        command: Option<AgentsCommand>,
    },
    /// Fleet sync: one node, the home, shares its instructions and skills;
    /// member nodes follow it into every profile (ADR 0012).
    Fleet {
        #[command(subcommand)]
        command: FleetCommand,
    },
    /// List the node's model routes (`model = "<route>"` in a profile;
    /// milestone 5): their providers, APIs and models, never a URL or key.
    Models,
    /// List sessions, newest first.
    Sessions,
    /// Start a session and attach to it.
    Run(Run),
    /// Attach to a session: its terminal, or its conversation.
    Attach {
        /// Session id, or a unique prefix of one.
        session: SessionSelector,
        /// Replay from this record (0 replays everything).
        #[arg(long, default_value = "0", value_parser = parse_seq)]
        from: EventSeq,
    },
    /// Send a line (terminal session) or a prompt (structured session).
    Send(Send),
    /// Search every session's messages, newest first (protocol 12): what
    /// was asked and what the agents answered, in sessions live, ended or
    /// archived.
    Search(Search),
    /// List an agent's own conversations, newest first (protocol 17), to
    /// import one: `vessel run AGENT --structured --resume ID --cwd DIR`.
    History(History),
    /// Print a session's log.
    Log(Log),
    /// Stop a session (its whole process group).
    Kill {
        /// Session id, or a unique prefix of one.
        session: SessionSelector,
    },
    /// Set this machine up, or check it again: the node's identity, the
    /// harnesses (offering to install what is missing), profiles for the
    /// other accounts found, their logins, and with `--service` the node
    /// as a per-user service. Safe to run again: it fills gaps and changes
    /// nothing else.
    Init(InitArgs),
    /// Serve the Vessel tools over MCP on stdio, as a session's agent. A
    /// node starts this for every session (the session's `vessel` MCP
    /// server), with its endpoint and the session's credential in the
    /// environment; it is not run by hand.
    Mcp,
    /// Change a session in place: its profile, its queue, answers to its
    /// agent, marks; start it again, copy, move or delete it.
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
    /// The node's projects: folders and repositories it works in
    /// (protocol 10, ADR 0027).
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    /// Worktrees: a folder and a branch of a project's own, per session.
    Worktree {
        #[command(subcommand)]
        command: WorktreeCommand,
    },
    /// A session's pull request on its forge: show it, open one, merge
    /// it, and send its feedback to the session (protocol 11, ADR 0029).
    Pr {
        #[command(subcommand)]
        command: PrCommand,
    },
    /// Shells beside sessions (protocol 11, ADR 0031): without a
    /// subcommand, open one (`--session` or `--project`) and attach.
    Shell(Shell),
    /// A project's scripts and services (`vessel.toml`), run once
    /// approved.
    Script {
        #[command(subcommand)]
        command: ScriptCommand,
    },
    /// The TCP ports the node's sessions and terminals listen on.
    Ports(Ports),
    /// Reported tokens and cost, by session, agent or machine.
    Usage {
        #[command(subcommand)]
        command: UsageCommand,
    },
    /// Schedules: prompts the node sends on its own, at times or every so
    /// often (protocol 11, ADR 0033).
    Schedule {
        #[command(subcommand)]
        command: ScheduleCommand,
    },
    /// Saved prompts: named texts, for a project or for the machine.
    Prompt {
        #[command(subcommand)]
        command: PromptCommand,
    },
    /// Serve the web UI on this machine and open it in the browser: every
    /// node in your book, their sessions, the fleet and devices. It runs
    /// until interrupted.
    Ui(UiArgs),
}

/// `vessel ui`'s default port (`vesseld` has 7077, its relay 7078).
pub(crate) const DEFAULT_UI_PORT: u16 = 7079;

/// `vessel ui`'s options.
#[derive(Debug, Args)]
pub(crate) struct UiArgs {
    /// The port to serve on; when the default is taken, any free one.
    #[arg(long, default_value_t = DEFAULT_UI_PORT)]
    pub(crate) port: u16,
    /// The loopback address to serve on: `127.0.0.1` or `::1`. The UI is for
    /// this machine only.
    #[arg(long, default_value = "127.0.0.1")]
    pub(crate) bind: std::net::IpAddr,
    /// Print the address instead of opening the browser.
    #[arg(long)]
    pub(crate) no_open: bool,
}

/// `vessel init`'s options.
#[derive(Debug, Args)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each is one command-line flag; `init::Options` folds them into a consent"
)]
pub(crate) struct InitArgs {
    /// Also install the node as a per-user service (systemd user unit on
    /// Linux, launchd agent on macOS) and start it.
    #[arg(long)]
    pub(crate) service: bool,
    /// Join a fleet: paste the link `vessel fleet invite` printed on the
    /// fleet's home (read from standard input, never the command line).
    #[arg(long)]
    pub(crate) join: bool,
    /// Answer yes to every question, except installs.
    #[arg(long, short = 'y')]
    pub(crate) yes: bool,
    /// With --yes, run the install commands for missing harnesses too.
    #[arg(long, requires = "yes")]
    pub(crate) install: bool,
    /// The home directory to look in for other accounts (tests).
    #[arg(long, env = "VESSEL_HOME", hide = true)]
    pub(crate) home: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum SessionCommand {
    /// Continue a structured session's conversation under another profile
    /// (another account of the same harness), for example at a usage
    /// limit. The target profile must be logged in.
    MoveProfile {
        /// Session id, or a unique prefix of one.
        session: SessionSelector,
        /// The profile to move to (`vessel profiles`); the harness's name
        /// is its default profile.
        profile: ProfileName,
        /// Move even in the middle of a turn, cancelling it.
        #[arg(long)]
        force: bool,
    },
    /// Copy a structured session's conversation to a node (this one by
    /// default) and one of its profiles, for a working directory there.
    /// The session goes on here; with --start the copy resumes there as a
    /// new session. Between turns only.
    Copy {
        #[command(flatten)]
        to: CopyTo,
        /// Resume the copy there as a new session.
        #[arg(long)]
        start: bool,
    },
    /// Move a structured session's conversation: copy it, resume it there
    /// as a new session, then stop it here. Between turns only.
    Move {
        #[command(flatten)]
        to: CopyTo,
    },
    #[command(flatten)]
    Conversation(ConversationCommand),
}

/// Where `vessel session copy|move` takes a conversation.
#[derive(Debug, Args)]
pub(crate) struct CopyTo {
    /// Session id, or a unique prefix of one.
    pub(crate) session: SessionSelector,
    /// The node to copy it to (as for --node); default: the session's own.
    #[arg(long)]
    pub(crate) to: Option<String>,
    /// The profile there (`vessel profiles`); default: the harness's
    /// default profile.
    #[arg(long)]
    pub(crate) profile: Option<ProfileName>,
    /// The working directory there, absolute; default: the session's own.
    #[arg(long)]
    pub(crate) cwd: Option<PathBuf>,
}

/// Longest pairing link lifetime, as the node allows (ADR 0010).
const MAX_TTL_SECS: u32 = 24 * 60 * 60;

/// `--ttl`: a number of seconds, minutes (`m`) or hours (`h`), from one
/// second to a day.
fn ttl_secs(text: &str) -> Result<u32, String> {
    let text = text.trim();
    let (number, unit) = match text.strip_suffix(['s', 'm', 'h']) {
        Some(number) => (number, text.chars().last()),
        None => (text, None),
    };
    let scale = match unit {
        Some('m') => 60,
        Some('h') => 60 * 60,
        _ => 1,
    };
    let invalid = || format!("`{text}` is not a duration like `90s`, `10m` or `2h`");
    let secs = number
        .parse::<u32>()
        .map_err(|_number| invalid())?
        .checked_mul(scale)
        .ok_or_else(invalid)?;
    if secs == 0 {
        return Err("a pairing link must work for at least a second".to_owned());
    }
    if secs > MAX_TTL_SECS {
        return Err("a pairing link works at most 24h".to_owned());
    }
    Ok(secs)
}

/// `vessel run`.
#[derive(Debug, Args)]
pub(crate) struct Run {
    /// Harness to run: claude-code (or claude), codex, gemini, opencode, or
    /// an agent added on the node (`vessel agents`).
    pub(crate) harness: HarnessRef,
    /// Drive it over ACP and show a conversation instead of its terminal UI
    /// (always, for an added agent).
    #[arg(long)]
    pub(crate) structured: bool,
    /// Model to request (structured mode).
    #[arg(long, requires = "structured")]
    pub(crate) model: Option<String>,
    /// Resume this harness session (structured mode): one of `vessel
    /// history`'s, imported.
    #[arg(long, requires = "structured")]
    pub(crate) resume: Option<String>,
    /// What to call the session.
    #[arg(long)]
    pub(crate) title: Option<SessionTitle>,
    /// Run under this profile of the node (`vessel profiles` lists them);
    /// the harness's default when absent.
    #[arg(long)]
    pub(crate) profile: Option<ProfileName>,
    /// Working directory on the node (defaults to the node user's home). A
    /// relative path is taken from the current directory when the node is
    /// on this machine, and refused for a remote node.
    #[arg(long, conflicts_with_all = ["project", "worktree"])]
    pub(crate) cwd: Option<PathBuf>,
    /// Run in this project's folder (`vessel project list`).
    #[arg(long, conflicts_with = "worktree")]
    pub(crate) project: Option<String>,
    /// With `--project`: a folder inside it, relative.
    #[arg(long, requires = "project", conflicts_with = "new_worktree")]
    pub(crate) dir: Option<String>,
    /// With `--project`: in a new worktree of it, after its setup.
    #[arg(long, requires = "project")]
    pub(crate) new_worktree: bool,
    /// Run in this worktree (`vessel worktree list`).
    #[arg(long)]
    pub(crate) worktree: Option<String>,
    /// Start it and print its id instead of attaching.
    #[arg(long)]
    pub(crate) detach: bool,
    /// Arguments passed to the harness verbatim (terminal mode), after `--`.
    #[arg(last = true, conflicts_with = "structured")]
    pub(crate) args: Vec<String>,
}

pub(crate) fn parse_seq(text: &str) -> Result<EventSeq, std::num::ParseIntError> {
    text.parse().map(EventSeq::new)
}

#[cfg(test)]
mod tests;
