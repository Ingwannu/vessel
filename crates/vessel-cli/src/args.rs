//! The command line, parsed once into typed values.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use vessel_client::BearerToken;
use vessel_proto::{EventSeq, HarnessRef, ProfileName};

use crate::selector::SessionSelector;

mod conversation;
mod fleet;
mod node;
mod nodes;
mod workspace;

pub(crate) use conversation::{ConversationCommand, QueueCommand, Send};
pub(crate) use fleet::{FleetCommand, MemoryCommand, SecretCommand};
pub(crate) use node::{DevicesCommand, Install, NodeCommand, Source, Update};
pub(crate) use nodes::{FromStdin, NodesCommand};
pub(crate) use workspace::{ProjectCommand, WorktreeCommand};

/// Vessel: drive agent harnesses on this and other machines.
#[derive(Debug, Parser)]
#[command(name = "vessel", version, about)]
pub(crate) struct Cli {
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
    /// Print a session's log.
    Log {
        /// Session id, or a unique prefix of one.
        session: SessionSelector,
        /// First record to print.
        #[arg(long, default_value = "0", value_parser = parse_seq)]
        from: EventSeq,
    },
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
    /// Drive it over ACP and show a conversation instead of its terminal UI.
    #[arg(long)]
    pub(crate) structured: bool,
    /// Model to request (structured mode).
    #[arg(long, requires = "structured")]
    pub(crate) model: Option<String>,
    /// Resume this harness session (structured mode).
    #[arg(long, requires = "structured")]
    pub(crate) resume: Option<String>,
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

fn parse_seq(text: &str) -> Result<EventSeq, std::num::ParseIntError> {
    text.parse().map(EventSeq::new)
}

#[cfg(test)]
mod tests;
