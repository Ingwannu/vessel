//! A feature-only real-node child exposes a lifecycle barrier, never a product flag.
//!
//! The controller still launches through its public API. This separate binary
//! keeps controlled exit out of the shipped node and avoids controller test hooks.

#[path = "local_node_fixture/control.rs"]
mod control;

use clap::{Parser, builder::TypedValueParser as _};
use futures_util::{SinkExt as _, StreamExt as _};
use std::io::Write as _;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;
use vessel_harness::{HarnessEnv, Lookup};
use vessel_node::{Config, DataDir, Node};
use vessel_rpc::Framed;

const CONTROL_DEADLINE: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Managed {
    Disabled,
    Enabled,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum Bind {
    None,
}

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    data_dir: PathBuf,
    #[arg(long, value_enum)]
    bind: Bind,
    #[arg(long, action = clap::ArgAction::SetTrue, value_parser = clap::builder::BoolValueParser::new().map(|on| if on { Managed::Enabled } else { Managed::Disabled }))]
    desktop_managed: Managed,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("fixture requires managed launch and explicit absolute scratch paths")]
    Paths,
    #[error("fixture control requires a loopback address")]
    Address,
    #[error("fixture environment is missing")]
    MissingEnvironment,
    #[error("fixture environment could not be decoded")]
    Environment(#[from] std::env::VarError),
    #[error("fixture address is invalid")]
    Parse(#[source] std::net::AddrParseError),
    #[error("fixture I/O failed")]
    Io(#[from] std::io::Error),
    #[error("fixture node failed")]
    Node(#[from] vessel_node::NodeError),
    #[error("fixture control framing failed")]
    Frame(#[from] vessel_rpc::FramedError),
    #[error("fixture control message failed")]
    Message(#[from] serde_json::Error),
    #[error("fixture control closed before stop")]
    Closed,
    #[error("fixture control timed out")]
    Deadline(#[from] tokio::time::error::Elapsed),
    #[error("fixture process identity is invalid")]
    Process(#[from] vessel_proc::InvalidProcessId),
}

fn path(name: &str) -> Result<PathBuf, Error> {
    let path = std::env::var_os(name)
        .map(PathBuf::from)
        .ok_or(Error::MissingEnvironment)?;
    if !path.is_absolute() {
        return Err(Error::Paths);
    }
    Ok(path)
}

fn config(args: Args) -> Result<Config, Error> {
    if !args.data_dir.is_absolute() || args.desktop_managed != Managed::Enabled {
        return Err(Error::Paths);
    }
    let mut config = Config::new(
        DataDir::new(args.data_dir),
        HarnessEnv::new(path(control::HOME_ENV)?),
    );
    config.harness_lookup = Lookup::new([path(control::PROGRAMS_ENV)?]);
    match args.bind {
        Bind::None => config.bind = None,
    }
    Ok(config)
}

fn announce(node: vessel_proto::NodeId) -> Result<(), Error> {
    let mut output = std::io::stdout().lock();
    output.write_all(
        vessel_proto::local_launch::Ready::new(node)
            .encode()
            .as_bytes(),
    )?;
    output.flush()?;
    Ok(())
}

async fn controlled(node: &Node) -> Result<(), Error> {
    let address: SocketAddr = std::env::var(control::CONTROL_ENV)?
        .parse()
        .map_err(Error::Parse)?;
    if !address.ip().is_loopback() {
        return Err(Error::Address);
    }
    let stream = tokio::net::TcpStream::connect(address).await?;
    let (reader, writer) = tokio::io::split(stream);
    let mut frames = Framed::new(reader, writer, control::FRAME_LIMIT);
    announce(node.info().id)?;
    frames
        .send(serde_json::to_string(&control::Notice {
            process: std::process::id().try_into()?,
            node: node.info().id,
        })?)
        .await?;
    let frame = tokio::time::timeout(CONTROL_DEADLINE, frames.next())
        .await?
        .ok_or(Error::Closed)??;
    match serde_json::from_str::<control::Command>(&frame)? {
        control::Command::Stop => Ok(()),
    }
}

async fn run(args: Args) -> Result<(), Error> {
    let node = Node::start(config(args)?).await?;
    let result = controlled(&node).await;
    // Even a vanished test driver must stop this real isolated node.
    let stopped = node.stop().await;
    result?;
    stopped?;
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
            tracing::error!(%error, "local-node fixture refused");
            ExitCode::FAILURE
        }
    }
}
