//! Graceful shutdown over the existing Admin-scoped `node.stop` (ADR 0064).
//! Process signalling and service management are separate later slices.

use vessel_client::book::Resolved;
use vessel_client::{CallError, NodeClient};
use vessel_node::NodeLock;
use vessel_proto::{ErrorCode, SessionInfo, StopWhen};

use crate::args::{Confirmation, Stop, StopPolicy, Target};
use crate::connect;
use crate::error::CliError;
use crate::{io, nodes, remote};

/// Respect live-session intent and wait on connection closure, never polling.
///
/// # Errors
/// Invalid timing, refused authorization, unconfirmed interruption, or expiry.
pub(crate) async fn stop(target: &Target, args: Stop) -> Result<(), CliError> {
    tokio::time::timeout(args.timeout.duration(), graceful(target, args.policy()))
        .await
        .map_err(CliError::StopTimedOut)?
}

async fn graceful(target: &Target, policy: StopPolicy) -> Result<(), CliError> {
    let (resolved, book) = nodes::resolve(target)?;
    // An explicit book token may belong to another local data directory.
    // Only the implicit local token establishes this directory as the home.
    if connect::is_here(&resolved) && resolved.token.is_none() && target.token.is_none() {
        let dir = connect::data_dir(target)?;
        tokio::task::spawn_blocking(move || require_running(&dir)).await??;
    }
    let connected = connect::connect_to(target, &resolved, &book).await?;
    let client = &connected.client;
    let live: Vec<_> = client
        .sessions()
        .await?
        .into_iter()
        .filter(|session| session.status.is_live())
        .collect();
    let when = choose(&resolved, &live, policy).await?;
    request(client, when).await?;
    if when == StopWhen::WhenIdle {
        io::out(b"the node is draining; it stops when the last live session ends\n").await?;
    } else if connect::is_here(&resolved) {
        // Closure is the existing lifecycle event. It proves the node closed
        // this connection, not that an external manager cannot start it again.
        client.closed().await;
        io::out(b"the node closed its shutdown connection\n").await?;
    } else {
        io::out(b"shutdown requested; remote process exit was not verified\n").await?;
    }
    Ok(())
}

fn require_running(dir: &vessel_node::DataDir) -> Result<(), CliError> {
    if NodeLock::holder(dir).map_err(CliError::NodeLock)?.is_some() {
        return Ok(());
    }
    let file = match std::fs::File::open(dir.lock_path()) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(CliError::NodeNotRunning);
        }
        Err(error) => return Err(error.into()),
    };
    // holder() returns no record for both an unused lock and an unreadable
    // owner file. Preserve that distinction rather than calling a live node
    // stopped when its record has been corrupted.
    match file.try_lock() {
        Ok(()) => {
            file.unlock()?;
            Err(CliError::NodeNotRunning)
        }
        Err(std::fs::TryLockError::WouldBlock) => Err(CliError::NodeOwnerUnavailable),
        Err(std::fs::TryLockError::Error(error)) => Err(error.into()),
    }
}

async fn choose(
    resolved: &Resolved,
    live: &[SessionInfo],
    policy: StopPolicy,
) -> Result<StopWhen, CliError> {
    if live.is_empty() {
        return Ok(StopWhen::Now);
    }
    match policy {
        StopPolicy::Protect => {
            io::out(remote::live_list(resolved, live).as_bytes()).await?;
            Err(CliError::StopHasLiveSessions)
        }
        StopPolicy::Drain => Ok(StopWhen::WhenIdle),
        StopPolicy::Interrupt(confirmation) => {
            match confirmation {
                Confirmation::Granted => {}
                Confirmation::Ask => {
                    if remote::confirm(resolved, live).await? == remote::ConfirmationReply::Declined
                    {
                        return Err(CliError::StopNotConfirmed);
                    }
                }
            }
            Ok(StopWhen::Now)
        }
    }
}

async fn request(client: &NodeClient, when: StopWhen) -> Result<(), CliError> {
    match client.stop_node(when).await {
        Ok(()) | Err(CallError::Disconnected) => Ok(()),
        Err(error) if error.code() == Some(ErrorCode::Unsupported) => {
            Err(CliError::StopUnsupported(error))
        }
        Err(error) => Err(error.into()),
    }
}
