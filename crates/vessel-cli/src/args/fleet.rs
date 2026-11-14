//! `vessel fleet`, `vessel memory` and `vessel secret`: a fleet's
//! membership, sync, the agents' memory and the node's secrets.

use std::path::PathBuf;

use clap::Subcommand;
use vessel_node::fleet::SkipGlob;
use vessel_proto::{NodeId, ProfileName, Scope, SecretName};

use super::ttl_secs;

/// `vessel fleet ...`. Without --node, join/leave/status retain offline
/// data-directory behavior. With --node they use authenticated RPC. Sync,
/// invite/members/published/forget always call the selected node;
/// init/restore stay local.
#[derive(Debug, Subcommand)]
pub(crate) enum FleetCommand {
    /// Make this node the fleet's home: write `fleet.toml`, naming the
    /// instructions file and skills directory every member gets.
    Init {
        /// The instructions file (`~/.codex/AGENTS.md`, say).
        #[arg(long)]
        instructions: Option<PathBuf>,
        /// The skills directory (`~/.codex/skills`, say).
        #[arg(long)]
        skills: Option<PathBuf>,
        /// Names to leave out (`*` and `?` within one name), such as
        /// skills another tool manages; repeatable.
        #[arg(long = "skip", value_name = "GLOB")]
        skip: Vec<SkipGlob>,
    },
    /// Make a single-use link for a node to join this home (needs the
    /// `admin` scope on the home). Prints the link, and a QR code on a
    /// terminal.
    Invite {
        /// Name for the member on the home (else it names itself).
        #[arg(long)]
        label: Option<String>,
        /// How long the link works: `90s`, `10m`, `2h` (at most `24h`).
        #[arg(long, default_value = "10m", value_parser = ttl_secs)]
        ttl: u32,
        /// Also let the member pass calls through this home to the fleet's
        /// other nodes (the `relay` scope), for its agents and devices.
        #[arg(long)]
        relay: bool,
    },
    /// Join the fleet whose home made the link: paste it at the prompt, or
    /// pipe it in with `--yes` (never on the command line).
    Join {
        /// Join without asking, after checking the key's fingerprint.
        #[arg(long)]
        yes: bool,
        /// What the home may do on this node, for calls from the fleet's
        /// other nodes: scopes separated by commas, or `none`.
        #[arg(long, default_value = "read,operate", value_parser = grant_scopes)]
        grant: GrantScopes,
    },
    /// Stop following the home. The files it wrote stay as they are.
    Leave,
    /// Ask the selected machine to sync now (needs admin). Success means
    /// accepted; status shows what was actually applied.
    Sync,
    /// This node's place in the fleet: its role, what it last applied, and
    /// the files it left alone and why.
    Status,
    /// The home's members and what each last applied (needs `admin`).
    Members,
    /// What the selected machine shares through its fleet, or last
    /// received: files, tool servers and the secrets they name, by name
    /// only, never contents or values (needs `admin`).
    Published,
    /// Let a member node grant again under a new key (it was reinstalled)
    /// or a new name (needs `admin` on the home).
    ///
    /// The home binds each node id to the key and name of its first grant
    /// for good; this forgets that binding and the node's grants, and
    /// records it. From then on any member device invited may take the id
    /// and name, and the first to claim them keeps them: invite only that
    /// node next (`vessel fleet invite`), join it, and invite others after
    /// it has joined. Every invitation still outstanding on the home is
    /// withdrawn and pairings in progress are refused, whoever they were
    /// for; make new ones then. Revoke any of the node's old devices that
    /// should not sync any more.
    Forget {
        /// The node's id (`vessel fleet members`, `list_nodes`).
        node: NodeId,
    },
    /// Bring back fleet files deleted here: one file, or every one below a
    /// directory. The node writes them again within a few seconds.
    Restore {
        /// The deleted file, or a directory.
        path: PathBuf,
    },
    /// Profiles' memory, copied between profiles and nodes on demand.
    Memory {
        #[command(subcommand)]
        command: MemoryCommand,
    },
    /// This node's secrets, which fill the fleet's MCP servers here
    /// (`secrets.toml`); they never leave this node.
    Secret {
        #[command(subcommand)]
        command: SecretCommand,
    },
}

/// What a member grants its home (`vessel fleet join --grant`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GrantScopes(pub(crate) Vec<Scope>);

pub(super) fn grant_scopes(text: &str) -> Result<GrantScopes, String> {
    if text.trim() == "none" {
        return Ok(GrantScopes(Vec::new()));
    }
    let mut scopes = Vec::new();
    for name in text.split(',').map(str::trim) {
        let scope = Scope::ALL
            .into_iter()
            .find(|scope| scope.as_str() == name)
            .ok_or_else(|| {
                format!("`{name}` is not a scope (read, operate, admin, sync, relay, or `none`)")
            })?;
        if !scopes.contains(&scope) {
            scopes.push(scope);
        }
    }
    Ok(GrantScopes(scopes))
}

/// `vessel fleet memory ...`.
#[derive(Debug, Subcommand)]
pub(crate) enum MemoryCommand {
    /// Copy a profile's memory to another profile or node (needs `admin`
    /// there). Files that differ are written beside the existing ones
    /// (`<name>.vessel-incoming`); nothing is overwritten.
    Copy {
        /// The node it comes from (as for --node); default: --node.
        #[arg(long)]
        from: Option<String>,
        /// The profile it comes from; default: Claude Code's default.
        #[arg(long)]
        from_profile: Option<ProfileName>,
        /// One project's memory (its working directory on that node);
        /// default: all of the profile's memory.
        #[arg(long)]
        from_project: Option<PathBuf>,
        /// The node it goes to (as for --node); default: --node.
        #[arg(long)]
        to: Option<String>,
        /// The profile it goes to; default: its harness's default profile.
        #[arg(long)]
        profile: Option<ProfileName>,
        /// The project it goes to, absolute; default: the same directory.
        #[arg(long, requires = "from_project")]
        project: Option<PathBuf>,
    },
}

/// `vessel fleet secret ...`.
#[derive(Debug, Subcommand)]
pub(crate) enum SecretCommand {
    /// Set a secret: typed at a prompt (not shown), or piped in; never on
    /// the command line, where other users can read it.
    Set {
        /// Its name, as the fleet's MCP servers refer to it.
        name: SecretName,
    },
    /// List the secrets set on this node (names only).
    List,
    /// Delete a secret.
    Delete {
        /// Its name.
        name: SecretName,
    },
    /// Copy a secret to another node of the fleet, which keeps it under
    /// the same name. That node must have granted the fleet's home `admin`
    /// (`vessel fleet join --grant read,operate,admin`).
    Copy {
        /// Its name.
        name: SecretName,
        /// The node's id, whole (`vessel fleet members` on the home, or
        /// `vessel status` there): a value goes only to the node named
        /// exactly, never to one a name or a prefix might also match.
        #[arg(long)]
        to: NodeId,
    },
}
