//! Explicit fake private-helper peer; never selected by a product node.

use std::{
    error::Error,
    io::{self, Write as _},
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

use tokio::io::AsyncWriteExt;
use vessel_plugin::wire;
use vessel_proto::plugins::{
    ClientDocument, HelperCall, HelperFailure, HelperProtocol, HelperToNode, HookName,
    HookSubscriptions, InvocationId, LogText, NodeToHelper,
};

const IDLE_INTERVAL: Duration = Duration::from_secs(1);

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let mode = arguments.first().map_or("echo", String::as_str);
    if mode == "grandchild" {
        loop {
            std::thread::sleep(IDLE_INTERVAL);
        }
    }
    if mode == "descendant" {
        let path = arguments
            .get(1)
            .ok_or_else(|| io::Error::other("missing fixture pid path"))?;
        descendant(Path::new(path))?;
    }
    if mode == "stall"
        && let Some(path) = arguments.get(1)
    {
        private_pid(Path::new(path), std::process::id())?;
    }
    let mut input = tokio::io::stdin();
    let mut output = tokio::io::stdout();
    let first = wire::read_node(&mut input).await?;
    if !matches!(first, Some(NodeToHelper::Initialize { .. })) {
        return Err(io::Error::other("fixture requires initialize first").into());
    }
    if mode == "malformed" {
        let body = br#"{"type":"PRIVATE-HELPER-CANARY"}"#;
        output
            .write_all(&u32::try_from(body.len())?.to_be_bytes())
            .await?;
        output.write_all(body).await?;
        output.flush().await?;
    } else if mode == "fatal" {
        wire::write_helper(
            &mut output,
            &HelperToNode::Fatal {
                diagnostic: "PRIVATE-HELPER-CANARY".parse()?,
            },
        )
        .await?;
    } else {
        if mode == "stderr-long" {
            stderr_long().await?;
        }
        ready(&mut output).await?;
    }
    while let Some(message) = wire::read_node(&mut input).await? {
        if mode == "stall" {
            continue;
        }
        match message {
            NodeToHelper::Initialize { .. } => {
                return Err(io::Error::other("duplicate initialize").into());
            }
            NodeToHelper::Request { request, call } => {
                let data = match call {
                    HelperCall::Invoke { input, .. } | HelperCall::Hook { input, .. } => input,
                };
                wire::write_helper(
                    &mut output,
                    &HelperToNode::Result {
                        request,
                        output: data,
                    },
                )
                .await?;
            }
            NodeToHelper::Cancel { request } => cancelled(&mut output, request).await?,
            NodeToHelper::Shutdown {} => {
                if mode == "stderr-tail" {
                    stderr_long().await?;
                }
                if mode != "ignore-shutdown" {
                    break;
                }
            }
        }
    }
    Ok(())
}

async fn ready(output: &mut tokio::io::Stdout) -> Result<(), Box<dyn Error>> {
    let client = ClientDocument::new(vec!["echo".parse()?], vec![], vec![])?;
    let hooks = HookSubscriptions::try_from(vec![HookName::AgentCreateBefore])?;
    wire::write_helper(
        output,
        &HelperToNode::Ready {
            protocol: HelperProtocol::V1,
            client,
            hooks,
        },
    )
    .await?;
    Ok(())
}

async fn cancelled(
    output: &mut tokio::io::Stdout,
    request: InvocationId,
) -> Result<(), Box<dyn Error>> {
    let diagnostic: LogText = "cancelled".parse()?;
    wire::write_helper(
        output,
        &HelperToNode::Error {
            request,
            kind: HelperFailure::Cancelled,
            diagnostic,
        },
    )
    .await?;
    Ok(())
}

async fn stderr_long() -> io::Result<()> {
    let mut stream = tokio::io::stderr();
    let chunk = vec![b'x'; vessel_proto::plugins::MAX_LOG_LINE_BYTES];
    for _ in 0..100 {
        stream.write_all(&chunk).await?;
    }
    stream.write_all(b"\nlast diagnostic\n").await?;
    stream.flush().await
}

fn descendant(path: &Path) -> io::Result<()> {
    // The fixture deliberately uses a normal inherited process group: making
    // another vessel-proc group here would test a child that intentionally escapes
    // this parent's group instead of proving descendant cleanup.
    let child = Command::new(std::env::current_exe()?)
        .arg("grandchild")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    private_pid(path, child.id())
}

fn private_pid(path: &Path, pid: u32) -> io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    let _configured = options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let _mode = options.mode(0o600);
    }
    let mut note = options.open(path)?;
    write!(note, "{pid}")?;
    Ok(())
}
