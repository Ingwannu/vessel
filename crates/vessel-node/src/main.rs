//! `vesseld`: the standalone node binary.
//!
//! - `vesseld [--bind ADDR|none]`: run the node until `Ctrl-C`, `SIGTERM`
//!   or a client's `node.stop`, then stop cleanly (sessions end as
//!   `Interrupted`, logs committed).
//! - `vesseld attach`: the proxy SSH runs (ADR 0007); stdout is the
//!   protocol stream, so it logs only warnings, to stderr.
//! - `vesseld version`: the version and target the installer checks.
//! - `vesseld cleanup`: delete installed binaries nothing needs (ADR 0008).
//! - `vesseld relay --bind ADDR --token-file PATH`: run a relay (ADR 0009)
//!   until `Ctrl-C` or `SIGTERM`; no data directory, nothing stored.

use std::io::Write as _;
use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;
use tokio_util::sync::CancellationToken;
use vessel_harness::HarnessEnv;
use vessel_node::{Config, DataDir, Node, VesselMcp};

mod daemon_args;
mod daemon_logging;
use daemon_args::{Args, Bind, Command, RelayArgs, Run};

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
    let attaching = matches!(args.command, Some(Command::Attach));
    daemon_logging::init(attaching);

    // A relay keeps nothing on disk: it needs no data directory (nor a home).
    let command = match args.command {
        Some(Command::Relay(relay_args)) => return relay(relay_args).await,
        command => command,
    };
    let Some(root) = args.data_dir.or_else(DataDir::default_root) else {
        tracing::error!("no home directory; pass --data-dir");
        return ExitCode::FAILURE;
    };
    let dir = DataDir::new(root);
    match command {
        None => run(dir, args.run).await,
        Some(Command::Attach) => ExitCode::from(vessel_node::attach::run(dir).await.code()),
        Some(Command::Version) => print(&format!("{}\n", vessel_node::VERSION_LINE)),
        Some(Command::Cleanup { keep }) => cleanup(dir, keep).await,
        Some(Command::Relay(relay_args)) => relay(relay_args).await,
    }
}

async fn relay(args: Box<RelayArgs>) -> ExitCode {
    let args = *args;
    let token = match vessel_relay::read_token_file(&args.token_file) {
        Ok(token) => token,
        Err(error) => {
            tracing::error!(error = %chain(&error), "relay failed to start");
            return ExitCode::FAILURE;
        }
    };
    if args.client_address_header.is_some() && args.trusted_proxies.is_empty() {
        // A header believed from anyone lets anyone pick their address and
        // slip every per-address limit.
        tracing::error!(
            "relay failed to start: --client-address-header needs --trusted-proxy, the \
             proxies whose header is believed"
        );
        return ExitCode::FAILURE;
    }
    if args.client_address_header.is_none() && !args.trusted_proxies.is_empty() {
        tracing::warn!("--trusted-proxy does nothing without --client-address-header");
    }
    let mut config = vessel_relay::RelayConfig::new(token);
    config.client_address_header = args.client_address_header;
    config.trusted_proxies = args.trusted_proxies;
    let trusted_proxies = config
        .trusted_proxies
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let limits = &mut config.limits;
    limits.max_nodes = args.max_nodes;
    limits.max_registering = args.max_registering;
    limits.max_clients = args.max_clients;
    limits.max_clients_per_node = args.max_clients_per_node;
    limits.max_bytes_per_second = args.max_bytes_per_second;
    limits.max_connections = args.max_connections;
    limits.max_unauthenticated = args.max_unauthenticated;
    limits.max_connections_per_address = args.max_connections_per_address;
    limits.max_registering_per_address = args.max_registering_per_address;
    limits.reserved_registering = args.reserved_registering;
    limits.max_clients_per_address = args.max_clients_per_address;
    limits.max_node_clients_per_address = args.max_node_clients_per_address;
    if let Err(unbalanced) = limits.check() {
        // One /48 could hold half of a pool: the relay could be denied by
        // a single host's network.
        tracing::error!(error = %unbalanced, "relay failed to start");
        return ExitCode::FAILURE;
    }
    tracing::info!(
        max_nodes = limits.max_nodes,
        max_registering = limits.max_registering,
        max_clients = limits.max_clients,
        max_clients_per_node = limits.max_clients_per_node,
        max_bytes_per_second = limits.max_bytes_per_second,
        max_connections = limits.max_connections,
        max_unauthenticated = limits.max_unauthenticated,
        max_connections_per_address = limits.max_connections_per_address,
        max_registering_per_address = limits.max_registering_per_address,
        reserved_registering = limits.reserved_registering,
        max_clients_per_address = limits.max_clients_per_address,
        max_node_clients_per_address = limits.max_node_clients_per_address,
        %trusted_proxies,
        "relay limits"
    );
    // Before `relay listening`: a stop sent after that line must stop it.
    let stop = vessel_node::StopSignals::listen();
    let listener = match tokio::net::TcpListener::bind(args.bind).await {
        Ok(listener) => listener,
        Err(error) => {
            tracing::error!(%error, address = %args.bind, "relay cannot listen");
            return ExitCode::FAILURE;
        }
    };
    if let Ok(address) = listener.local_addr() {
        tracing::info!(%address, "relay listening");
    }
    let shutdown = CancellationToken::new();
    let mut serving = tokio::spawn(vessel_relay::serve(listener, config, shutdown.clone()));
    let served = tokio::select! {
        () = stop.stopped() => {
            tracing::info!("stopping");
            shutdown.cancel();
            serving.await
        }
        served = &mut serving => served,
    };
    match served {
        Ok(Ok(())) => ExitCode::SUCCESS,
        Ok(Err(error)) => {
            tracing::error!(%error, "relay failed");
            ExitCode::FAILURE
        }
        Err(error) => {
            tracing::error!(%error, "relay task failed");
            ExitCode::FAILURE
        }
    }
}

async fn run(dir: DataDir, args: Run) -> ExitCode {
    // The one place `vesseld` reads the account it runs as: the binary,
    // never the library (`HarnessEnv`).
    let Some(user_home) = dirs::home_dir() else {
        tracing::error!("this user has no home directory; set HOME");
        return ExitCode::FAILURE;
    };
    let env = HarnessEnv::from_vars(user_home, |name| std::env::var_os(name));
    let mut config = Config::new(dir, env).with_local_limits(|name| std::env::var_os(name));
    // Terminals (protocol 11): the shell and the credentials, from this
    // binary's own environment.
    config.terminals = vessel_node::TerminalsConfig::detect(
        |name| std::env::var_os(name),
        std::env::vars_os().map(|(name, _)| name),
        |name| config.harness_lookup.find(name),
    );
    config.bind = match args.bind {
        Bind::Tcp(address) => Some(address),
        Bind::None => None,
    };
    config.name = args.name;
    config.lsp.registry.search_path = std::env::var_os("PATH").unwrap_or_default();
    config.lsp.registry.system_root = std::env::var_os("SystemRoot");
    if let Some(path) = args.lsp_config
        && let Err(error) = vessel_node::load_lsp_config(&mut config, &path).await
    {
        tracing::error!(%error, "LSP configuration refused");
        return ExitCode::FAILURE;
    }
    config.lsp_roots.extend(
        args.lsp_roots
            .into_iter()
            .map(|path| vessel_node::LspRoot {
                path,
                full_analysis: false,
            })
            .chain(
                args.lsp_full_roots
                    .into_iter()
                    .map(|path| vessel_node::LspRoot {
                        path,
                        full_analysis: true,
                    }),
            ),
    );
    config.agent_handshake_timeout = Duration::from_secs(args.agent_handshake_timeout);
    if let Some(minutes) = args.park_after {
        config.park_after = (minutes > 0).then(|| Duration::from_secs(u64::from(minutes) * 60));
        config.park_after_given = true;
    }
    config.relays = args.relays;
    config.relay_token_file = args.relay_token_file;
    config.public = args.public;
    config.client_address_header = args.client_address_header;
    config.trusted_proxies = args.trusted_proxies;
    config.vessel_mcp = vessel_beside_me();
    // Pull requests (ADR 0029): this environment's tokens and the
    // machine's `gh`, found here (the library reads no environment).
    config.forge =
        vessel_node::ForgeSetup::found(|name| std::env::var_os(name), &config.harness_lookup);

    // Before the node starts (and says it listens), so neither a hang-up
    // nor a stop sent once it is ready can kill it.
    #[cfg(unix)]
    let _hangups = args.detached.then(ignore_hangups).flatten();
    let stop = vessel_node::StopSignals::listen();
    let node = match Node::start(config).await {
        Ok(node) => node,
        Err(error) => {
            tracing::error!(error = %chain(&error), "node failed to start");
            return ExitCode::FAILURE;
        }
    };
    if args.detached {
        let _written = print(&format!(
            "{} {}\n",
            vessel_node::attach::READY,
            std::process::id()
        ));
    }
    tokio::select! {
        () = stop.stopped() => {}
        () = node.stop_requested() => {}
    }
    tracing::info!("stopping");
    match node.stop().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(error = %chain(&error), "node failed");
            ExitCode::FAILURE
        }
    }
}

/// A node started by `attach` must survive the end of the SSH session that
/// started it. Listening for `SIGHUP` replaces its default action (exit)
/// for the rest of the process; the stream itself is never read.
#[cfg(unix)]
fn ignore_hangups() -> Option<tokio::signal::unix::Signal> {
    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())
        .inspect_err(|error| tracing::warn!(%error, "cannot ignore SIGHUP"))
        .ok()
}

async fn cleanup(dir: DataDir, keep: usize) -> ExitCode {
    let cleaned =
        tokio::task::spawn_blocking(move || vessel_node::cleanup::cleanup(&dir, keep)).await;
    match cleaned {
        Ok(Ok(deleted)) => {
            let mut text = String::new();
            for path in &deleted {
                text.push_str(&path.display().to_string());
                text.push('\n');
            }
            print(&text)
        }
        Ok(Err(error)) => {
            tracing::error!(error = %chain(&error), "cleanup failed");
            ExitCode::FAILURE
        }
        Err(error) => {
            tracing::error!(%error, "cleanup task failed");
            ExitCode::FAILURE
        }
    }
}

/// Write `text` to stdout and flush it.
fn print(text: &str) -> ExitCode {
    let mut stdout = std::io::stdout().lock();
    match stdout
        .write_all(text.as_bytes())
        .and_then(|()| stdout.flush())
    {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(%error, "cannot write to stdout");
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

/// `vessel mcp` for sessions' agents (ADR 0013): the `vessel` installed
/// beside this `vesseld`, if there is one (releases ship both together);
/// otherwise sessions get no `vessel` server.
fn vessel_beside_me() -> Option<VesselMcp> {
    let me = std::env::current_exe().ok()?;
    let vessel = me.with_file_name(format!("vessel{}", std::env::consts::EXE_SUFFIX));
    vessel.is_file().then(|| VesselMcp::vessel(vessel))
}
