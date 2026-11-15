//! Graceful shutdown over the existing Admin-scoped `node.stop` (ADR 0064).
//! Local completion additionally requires the authenticated directory lock.
//! Process signalling and service management remain separate slices.

mod local_read;
mod lock_wait;
mod observe;
mod service_probe;

use self::local_read::{Kind, Pending};

use vessel_client::book::Resolved;
use vessel_client::{CallError, NodeClient};
use vessel_node::{KernelProbe, NodeLock};
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
    let limit = args.timeout;
    tokio::time::timeout(limit.duration(), graceful(target, args.policy()))
        .await
        .map_err(|source| CliError::StopTimedOut { limit, source })?
}

async fn graceful(target: &Target, policy: StopPolicy) -> Result<(), CliError> {
    let (resolved, book) = nodes::resolve(target)?;
    // An explicit book token may belong to another local data directory.
    // Only the implicit local token establishes this directory as the home.
    if connect::is_here(&resolved) && resolved.token.is_none() && target.token.is_none() {
        let dir = connect::data_dir(target)?;
        let mut pending = Pending::spawn(
            Kind::Preflight,
            move || require_running(&dir),
            || tracing::debug!("node preflight completed after its command ended"),
        )?;
        pending.wait().await.map_err(CliError::PreflightReader)??;
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
    let mut observer = observe::Observer::bind(target, &resolved, client, when).await?;
    let service = service_probe::inspect(observer.directory()).await?;
    if let Some(warning) = service.warning() {
        io::note(warning).await?;
    }
    request(client, when).await?;
    let completion = observer.completed().await?;
    io::out(completion.message().as_bytes()).await?;
    Ok(())
}

fn require_running(dir: &vessel_node::DataDir) -> Result<(), CliError> {
    if NodeLock::holder(dir).map_err(CliError::NodeLock)?.is_some() {
        return Ok(());
    }
    match NodeLock::kernel_holder(dir).map_err(CliError::NodeLock)? {
        KernelProbe::Found(_) => return Err(CliError::NodeOwnerUnavailable),
        KernelProbe::NotVisible | KernelProbe::Unsupported => {}
    }
    // Absence from procfs is not proof of availability (namespaces and
    // filesystem emulation). Preserve the actual OS test as the fallback.
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
