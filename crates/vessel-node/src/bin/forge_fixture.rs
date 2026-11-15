//! Test-only node launcher (ADR 0060), deliberately separate from vesseld.
//! It shares the real node/admission/store and the existing `FakeForge`;
//! explicit scratch homes/programs and loopback listeners prevent accidental
//! fallback to the operator's harness paths or a network forge account.
use std::io::Write as _;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use vessel_forge::fake::seed::{Seed, SeedError};
use vessel_harness::{HarnessEnv, Lookup};
use vessel_node::{Config, DataDir, ForgeSetup, Node, NodeError, VesselMcp};
use vessel_proto::RouteUrl;

#[derive(Debug, Parser)]
#[command(
    name = "vessel-forge-fixture",
    version,
    about = "A test-only Vessel node with a file-seeded in-memory forge"
)]
struct Args {
    /// Version-1 JSON seed, read once before listening.
    #[arg(long)]
    seed: PathBuf,
    /// Disposable node state; no operator defaults.
    #[arg(long)]
    data_dir: PathBuf,
    /// Explicit disposable harness home; never inferred from this account.
    #[arg(long)]
    home: PathBuf,
    /// Search only this directory for fake harnesses and fixture tools.
    #[arg(long)]
    programs: PathBuf,
    /// Loopback listener; port zero is allowed and readiness reports it.
    #[arg(long)]
    bind: SocketAddr,
    /// Fixture name shown to clients.
    #[arg(long)]
    name: Option<String>,
    /// Explicit loopback ws/wss route, including its scheme (repeatable).
    #[arg(long = "public")]
    public: Vec<RouteUrl>,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("forge fixture nodes must bind and advertise loopback only")]
    NotLoopback,
    #[error("could not prepare fixture paths or readiness output")]
    Io(#[from] std::io::Error),
    #[error("could not parse a fixture public route")]
    Route(#[source] url::ParseError),
    #[error(transparent)]
    Seed(#[from] SeedError),
    #[error("fixture preparation task failed")]
    Task(#[from] tokio::task::JoinError),
    #[error("fixture node failed")]
    Node(#[from] NodeError),
    #[error("fixture node has no TCP listener")]
    NoListener,
}

fn absolute(path: PathBuf) -> Result<PathBuf, std::io::Error> {
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn prepare(args: Args) -> Result<Config, Error> {
    if !args.bind.ip().is_loopback() {
        return Err(Error::NotLoopback);
    }
    for route in &args.public {
        let url = url::Url::parse(route.as_str()).map_err(Error::Route)?;
        let local = match url.host() {
            Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
            Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
            Some(url::Host::Domain(name)) => name.eq_ignore_ascii_case("localhost"),
            None => false,
        };
        if !local || !url.username().is_empty() || url.password().is_some() {
            return Err(Error::NotLoopback);
        }
    }
    let forge = Seed::load(&args.seed)?.into_forge();
    let home = absolute(args.home)?;
    std::fs::create_dir_all(&home)?;
    let mut config = Config::new(
        DataDir::new(absolute(args.data_dir)?),
        HarnessEnv::new(home),
    );
    config.harness_lookup = Lookup::new([absolute(args.programs)?]);
    config.bind = Some(args.bind);
    config.name = args.name;
    config.public = args.public;
    config.terminals = vessel_node::TerminalsConfig::detect(
        |name| std::env::var_os(name),
        std::env::vars_os().map(|(name, _)| name),
        |name| config.harness_lookup.find(name),
    );
    // No environment token/gh discovery: custom replaces every host's
    // provider and the supplied file can never request an HTTP fallback.
    config.forge = ForgeSetup {
        custom: Some(forge),
        ..ForgeSetup::default()
    };
    // Binary-specific process discovery stays here, not in the library.
    config.vessel_mcp = std::env::current_exe().ok().and_then(|me| {
        let sibling = me.with_file_name(format!("vessel{}", std::env::consts::EXE_SUFFIX));
        sibling.is_file().then(|| VesselMcp::vessel(sibling))
    });
    Ok(config)
}

fn ready(address: SocketAddr) -> Result<(), std::io::Error> {
    let mut out = std::io::stdout().lock();
    out.write_all(format!("FIXTURE_READY {address}\n").as_bytes())?;
    out.flush()
}

async fn run(args: Args) -> Result<(), Error> {
    let config = tokio::task::spawn_blocking(move || prepare(args)).await??;
    let node = Node::start(config).await?;
    let address = node.address().ok_or(Error::NoListener)?;
    if let Err(error) = ready(address) {
        if node.stop().await.is_err() {
            tracing::error!("fixture shutdown failed after readiness output failed");
        }
        return Err(Error::Io(error));
    }
    tokio::select! { ()=vessel_node::signal()=>{}, ()=node.stop_requested()=>{} }
    node.stop().await?;
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(false)
        .init();
    match run(args).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // Do not print the source chain: a JSON parser's own diagnostic
            // can quote a malformed token or unknown field from the file.
            tracing::error!(error=%error,"forge fixture refused");
            ExitCode::FAILURE
        }
    }
}
