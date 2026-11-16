//! Explicit test-only node, with real policy/admission/cache and scratch-only recorded usage.
//! Installed vesseld has no seed argument or ambient fixture environment switch.
use clap::Parser;
use std::{
    io::{self, Read, Write as _},
    net::SocketAddr,
    path::{Path, PathBuf},
    process::ExitCode,
};
use vessel_harness::{
    AccountSourceAccess, AccountUsageProbe, HarnessEnv, LimitProbe, Lookup, RecordedUsageFormat,
};
use vessel_node::{Config, DataDir, Node};
use vessel_proto::{RouteUrl, usage_reports::BuiltinUsageSource};
#[path = "usage_fixture/gate.rs"]
mod gate;
const MARKER: &[u8] = b"vessel-usage-fixture-v1\n";
const MARKER_BYTES: u64 = 64;
#[derive(Debug, Parser)]
#[command(
    name = "vessel-usage-fixture",
    about = "Scratch-only real Vessel node for usage report UI tests"
)]
struct Args {
    /// Existing isolated root prepared by scripts/usage-fixture.py; no defaults.
    #[arg(long)]
    root: PathBuf,
    /// Loopback only, with zero permitted for an ephemeral listener.
    #[arg(long)]
    bind: SocketAddr,
    /// Loopback ws/wss route that pairing links advertise (repeatable), so a
    /// native capture app pairs through the CLI pipe exactly as with a fleet
    /// node. Without one, links carry no direct route.
    #[arg(long = "public")]
    public: Vec<RouteUrl>,
}
#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("usage fixtures require a marked isolated root and non-live loopback listener")]
    Unsafe,
    #[error("could not parse a usage fixture public route")]
    Route(#[source] url::ParseError),
    #[error("usage fixture filesystem setup or readiness output failed")]
    Io(#[from] io::Error),
    #[error("usage fixture setup task failed")]
    Task(#[from] tokio::task::JoinError),
    #[error("usage fixture node failed")]
    Node(#[from] vessel_node::NodeError),
}
fn within(root: &Path, name: &str) -> Result<PathBuf, Error> {
    let path = root.join(name).canonicalize()?;
    if !path.starts_with(root) {
        return Err(Error::Unsafe);
    }
    Ok(path)
}
/// Same rule as the forge fixture: loopback hosts only, no credentials, and
/// never a live preview port.
fn loopback_route(route: &RouteUrl) -> Result<(), Error> {
    let url = url::Url::parse(route.as_str()).map_err(Error::Route)?;
    let local = match url.host() {
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        Some(url::Host::Domain(name)) => name.eq_ignore_ascii_case("localhost"),
        None => false,
    };
    let live = matches!(url.port(), Some(7077 | 18_220 | 18_221));
    if !local || live || !url.username().is_empty() || url.password().is_some() {
        return Err(Error::Unsafe);
    }
    Ok(())
}
fn prepare(args: &Args) -> Result<Config, Error> {
    if !args.bind.ip().is_loopback() || matches!(args.bind.port(), 7077 | 18_220 | 18_221) {
        return Err(Error::Unsafe);
    }
    for route in &args.public {
        loopback_route(route)?;
    }
    let root = args.root.canonicalize()?;
    let mut marker = Vec::new();
    let _read = std::fs::File::open(root.join("usage-fixture.marker"))?
        .take(MARKER_BYTES)
        .read_to_end(&mut marker)?;
    if marker != MARKER {
        return Err(Error::Unsafe);
    }
    let home = within(&root, "home")?;
    let data = within(&root, "data")?;
    let programs = within(&root, "bin")?;
    let body = within(&root, "source-report.json")?;
    let mut config = Config::new(DataDir::new(data), HarnessEnv::new(home.clone()));
    config.bind = Some(args.bind);
    config.name = Some("Recorded usage fixture".to_owned());
    config.public.clone_from(&args.public);
    config.harness_lookup = Lookup::new([programs]);
    let gate_root = root.clone();
    config.usage_sources = AccountUsageProbe::Recorded {
        root: root.clone(),
        access: AccountSourceAccess::new(home, |_name| None).with_cursor_reader(move |_path| {
            gate::wait(&gate_root)?;
            Ok(None)
        }),
        source: BuiltinUsageSource::Cursor,
        body,
        status: 200,
        retry_after: None,
        format: RecordedUsageFormat::Report,
    };
    let legacy = root.join("legacy-body.json");
    if legacy.exists() {
        config.limits = LimitProbe::Recorded {
            root: root.clone(),
            body: within(&root, "legacy-body.json")?,
            status: 200,
            retry_after: None,
        };
    }
    Ok(config)
}
fn ready(address: SocketAddr) -> Result<(), Error> {
    let mut out = io::stdout().lock();
    writeln!(out, "USAGE_FIXTURE_READY {address}")?;
    out.flush()?;
    Ok(())
}
async fn run(args: Args) -> Result<(), Error> {
    let stopping = vessel_node::StopSignals::listen();
    let config = tokio::task::spawn_blocking(move || prepare(&args)).await??;
    let node = Node::start(config).await?;
    let Some(address) = node.address() else {
        node.stop().await?;
        return Err(Error::Unsafe);
    };
    if let Err(error) = ready(address) {
        let _stopped = node.stop().await;
        return Err(error);
    }
    tokio::select! {()=stopping.stopped()=>{},()=node.stop_requested()=>{}}
    node.stop().await?;
    Ok(())
}
#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_target(false)
        .init();
    match run(Args::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(error=%error,"usage fixture refused");
            ExitCode::FAILURE
        }
    }
}
