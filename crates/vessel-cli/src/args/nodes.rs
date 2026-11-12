//! `vessel nodes`: the node book this machine keeps, and pairing with a
//! node by its link.

use std::path::PathBuf;

use clap::Subcommand;
use vessel_client::RemotePath;
use vessel_client::book::NodeName;

/// `--pair` reads its link from standard input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FromStdin;

/// `--pair` or `--pair -`, and nothing else: a link on the command line is
/// readable by every user of the machine (`ps`, `/proc`) and stays in the
/// shell's history, so someone else could redeem it first.
pub(super) fn link_from_stdin(text: &str) -> Result<FromStdin, String> {
    if text == "-" {
        return Ok(FromStdin);
    }
    Err(
        "the pairing link is read from standard input, never from the command line, where \
         other users of this machine can read it and your shell history keeps it. Make a new \
         link with `vessel node pair` (this one has been exposed), then run `vessel nodes add \
         NAME --pair` and paste it"
            .to_owned(),
    )
}

#[derive(Debug, Subcommand)]
pub(crate) enum NodesCommand {
    /// Add a node: `vessel nodes add gpu-box me@gpu-box`, a websocket node
    /// with `--token-file`, or pair with one: `vessel nodes add gpu-box
    /// --pair`, then paste the link `vessel node pair` printed.
    Add {
        /// Its name (letters, digits, `.`, `_`, `-`).
        name: NodeName,
        /// How to reach it: `[user@]host` or `ssh://[user@]host[:port]` over
        /// SSH, or a `ws://` URL.
        #[arg(required_unless_present = "pair")]
        address: Option<String>,
        /// A websocket node's token file.
        #[arg(long)]
        token_file: Option<PathBuf>,
        /// An SSH node's data directory on that machine (default
        /// `~/.vessel`).
        #[arg(long)]
        remote_data_dir: Option<RemotePath>,
        /// Pair with the node: the link is read from standard input (pasted
        /// at the prompt, or piped), never from the command line.
        #[arg(
            long,
            num_args = 0..=1,
            default_missing_value = "-",
            value_name = "-",
            value_parser = link_from_stdin,
            conflicts_with_all = ["address", "token_file", "remote_data_dir"],
        )]
        pair: Option<FromStdin>,
        /// Pair without asking to confirm the node's key (needed when the
        /// link is piped in). Only with `--pair`.
        #[arg(long)]
        yes: bool,
        /// Make it the default node.
        #[arg(long)]
        default: bool,
    },
    /// List the nodes in the book.
    List,
    /// Show one node.
    Show {
        /// Its name.
        name: String,
    },
    /// Remove a node from the book.
    Remove {
        /// Its name.
        name: String,
    },
    /// Give a node in the book another name (the default follows it).
    Rename {
        /// Its name now.
        name: String,
        /// The new name.
        to: NodeName,
    },
    /// Set (or with `--none`, clear) the default node.
    Default {
        /// The node.
        #[arg(required_unless_present = "none")]
        name: Option<String>,
        /// No default: commands then use the node on this machine.
        #[arg(long, conflicts_with = "name")]
        none: bool,
    },
}
