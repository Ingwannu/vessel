//! `vessel`: the command-line client for Vessel nodes.
//!
//! Commands: `node start`, `node stop`, `node install`, `nodes`, `status`, `harnesses`,
//! `sessions`, `run`, `attach`, `send`, `log`, `kill`, `session` (a
//! session's profile, queue, answers, marks, starting again, deletion). Everything goes through
//! `vessel-client`; this crate only parses arguments, resolves the token,
//! and turns results into text. Output goes to stdout, notes and errors to
//! stderr, so `vessel log ID > file` captures exactly the session's output.

mod agents;
mod args;
mod attach;
mod automation;
mod commands;
mod connect;
mod conversation;
mod devices;
mod diagnostics;
mod error;
mod file_terminals;
mod fleet;
mod forge;
mod formatted_models;
mod history;
mod inbox;
mod init;
mod io;
mod log;
mod node_ctl;
mod nodes;
mod pair;
mod parity;
mod plugin;
mod presentation;
mod project_commands;
mod remote;
mod render;
mod search;
mod secret;
mod selector;
mod terminals;
mod transfer;
mod ui;
mod usage;
mod view;
mod workspace;

use std::process::ExitCode;

use clap::Parser as _;

use crate::args::{Cli, Command};

/// The stack of the thread the CLI runs on.
const STACK: usize = 8 * 1024 * 1024;

/// Run the CLI on a thread with an 8 MiB stack.
///
/// [Decision Log]
/// - Purpose: in debug builds every `async fn`'s `poll` gets a stack frame
///   holding the temporaries of all its arms, and a command nests several
///   (`main`, `commands::run`, the command, `connect`, ...). The main
///   thread's stack is 1 MiB on Windows, and it overflowed there twice:
///   when the profile commands were added, then the fleet commands.
///   Boxing the command future moved the futures to the heap, not those
///   frames.
/// - Choice: a thread of its own with the 8 MiB Linux gives a main thread,
///   building the same runtime `#[tokio::main]` built and blocking on the
///   CLI there. Tokio's worker threads keep their size; a command's own
///   future runs on this thread.
/// - Alternatives: boxing every arm of every command (each new command
///   would need remembering it); a release-only workaround (debug builds
///   are what the tests run).
fn main() -> ExitCode {
    let spawned = std::thread::Builder::new()
        .name("vessel".to_owned())
        .stack_size(STACK)
        .spawn(|| {
            match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime.block_on(cli()),
                Err(error) => {
                    complain(&format!("error: cannot start the async runtime: {error}\n"));
                    ExitCode::FAILURE
                }
            }
        });
    match spawned.map(std::thread::JoinHandle::join) {
        Ok(Ok(code)) => code,
        // The panic hook has printed what happened.
        Ok(Err(_panicked)) => ExitCode::FAILURE,
        Err(error) => {
            complain(&format!("error: cannot start a thread: {error}\n"));
            ExitCode::FAILURE
        }
    }
}

/// Write `text` to stderr before (or instead of) the runtime.
fn complain(text: &str) {
    let _ignored = std::io::Write::write_all(&mut std::io::stderr(), text.as_bytes());
}

async fn cli() -> ExitCode {
    let cli = Cli::parse();
    let diagnostic_output = matches!(
        &cli.command,
        Command::Node {
            command: args::NodeCommand::Diagnostics { .. }
        }
    );
    // A node shows its lifecycle; client commands only warnings.
    let default_level = match cli.command {
        Command::Node { .. } => "info",
        Command::Nodes { .. }
        | Command::Status
        | Command::Harnesses
        | Command::Profiles { .. }
        | Command::Agents { .. }
        | Command::Fleet { .. }
        | Command::Models
        | Command::Plugin { .. }
        | Command::Sessions
        | Command::Run(_)
        | Command::Attach { .. }
        | Command::Send(_)
        | Command::Log(_)
        | Command::Search(_)
        | Command::History(_)
        | Command::Kill { .. }
        | Command::Session { .. }
        | Command::Project { .. }
        | Command::Worktree { .. }
        | Command::Pr { .. }
        | Command::Shell(_)
        | Command::Script { .. }
        | Command::Ports(_)
        | Command::Usage { .. }
        | Command::Schedule { .. }
        | Command::Prompt { .. }
        | Command::Init(_)
        | Command::Mcp
        | Command::Inbox { .. }
        | Command::Ui(_) => "warn",
    };
    tracing_subscriber::fmt()
        .with_env_filter(
            if diagnostic_output { "off".into() } else {
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_unset| default_level.into())
            },
        )
        .with_writer(std::io::stderr)
        // Colours only for a person at a terminal, not for log files.
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stderr()))
        .init();

    // Boxed: the future of every command together is large (see `main`).
    match Box::pin(commands::run(cli)).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // Diagnostic errors have a safe root Display. Their typed causes
            // remain internal; normal commands keep the established chain.
            let text = if diagnostic_output {
                error.to_string()
            } else {
                chain(&error)
            };
            if io::note(&format!("error: {text}\n")).await.is_err() {
                tracing::error!("could not report an error on stderr");
            }
            ExitCode::FAILURE
        }
    }
}

/// The error and its causes on one line.
fn chain(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}
