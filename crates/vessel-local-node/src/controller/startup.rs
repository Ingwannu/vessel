//! One start deadline, with the retained child outside cancelable proof work.
//!
//! [Decision Log]
//! Failed proof must not return before a fire-and-forget Drop happens to reap.
//! The same local launch owner requests graceful cleanup and checks its wait;
//! uncertainty preserves the original failure and drops only that owner at once.
//! Success adds no wait. Caller cancellation retains the ordinary Drop fallback.

use super::{Lease, LocalNodeError, Opened, Operation};
use crate::launch::{Owned, Readiness, START_DEADLINE};
use crate::{Launch, LogReader};
use std::io::ErrorKind;
use std::time::Duration;
use vessel_client::{LocalConnectError, NodeClient};
use vessel_proto::{NodeId, NodeState};

async fn timed<T>(
    remaining: Duration,
    operation: impl Future<Output = Result<T, LocalNodeError>>,
) -> Result<T, LocalNodeError> {
    tokio::time::timeout(remaining, operation)
        .await
        .map_err(|source| LocalNodeError::Deadline {
            operation: Operation::Start,
            source,
        })?
}

pub(super) async fn open(
    launch: &Launch,
    on_logs: impl FnOnce(LogReader),
) -> Result<Opened, LocalNodeError> {
    let began = tokio::time::Instant::now();
    let (_permit, attached) = timed(START_DEADLINE, prepare(launch)).await?;
    if let Some(client) = attached {
        return Ok(Opened {
            lease: Lease::Attached(client),
            logs: None,
            #[cfg(unix)]
            reaping: None,
        });
    }
    // This owner is outside the timed future, so a proof timeout cannot drop
    // it before this function can request and observe original-child cleanup.
    let started = Owned::start(launch)?;
    let mut child = started.child;
    let reader = started.reader;
    #[cfg(unix)]
    let reaping = started.reaping;
    on_logs(child.logs());
    let remaining = START_DEADLINE.saturating_sub(began.elapsed());
    let client = match timed(remaining, prove(launch, &mut child, reader)).await {
        Ok(client) => client,
        Err(start) => {
            let cleanup = child.cleanup().await;
            // On either outcome release this owner NOW. If checked cleanup was
            // unconfirmed, Drop requests hard kill; it does not prove settlement.
            drop(child);
            return match cleanup {
                Ok(_exit) => Err(start),
                Err(cleanup) => Err(LocalNodeError::CleanupUnconfirmed {
                    start: Box::new(start),
                    cleanup,
                }),
            };
        }
    };
    let logs = Some(child.logs());
    Ok(Opened {
        lease: Lease::Owned {
            client,
            child: Box::new(child),
        },
        logs,
        #[cfg(unix)]
        reaping: Some(reaping),
    })
}

async fn prepare(
    launch: &Launch,
) -> Result<(crate::node_files::StartPermit, Option<NodeClient>), LocalNodeError> {
    let permit = launch.home().start_permit().await?;
    if let Some(identity) = launch.home().identity().await? {
        match connect(launch, &identity).await {
            Ok(client) => {
                serving(&client).await?;
                return Ok((permit, Some(client)));
            }
            Err(LocalNodeError::Connect(LocalConnectError::Unreachable { source, .. }))
                if matches!(
                    source.kind(),
                    ErrorKind::NotFound | ErrorKind::ConnectionRefused
                ) => {}
            Err(error) => return Err(error),
        }
    }
    Ok((permit, None))
}

async fn connect(
    launch: &Launch,
    identity: &crate::node_files::Identity,
) -> Result<NodeClient, LocalNodeError> {
    let client = NodeClient::connect_local(
        identity.endpoint.path(),
        &identity.credential,
        launch.client().clone(),
    )
    .await
    .map_err(LocalNodeError::Connect)?;
    expected_node(client, identity.node)
}

fn expected_node(client: NodeClient, expected: NodeId) -> Result<NodeClient, LocalNodeError> {
    if client.node().id != expected {
        return Err(LocalNodeError::Identity {
            expected,
            found: client.node().id,
        });
    }
    Ok(client)
}

async fn serving(client: &NodeClient) -> Result<(), LocalNodeError> {
    let info = client.node_info().await.map_err(LocalNodeError::Call)?;
    if info.id != client.node().id {
        return Err(LocalNodeError::Identity {
            expected: client.node().id,
            found: info.id,
        });
    }
    match info.state {
        NodeState::Serving => Ok(()),
        NodeState::Draining => Err(LocalNodeError::Draining),
    }
}

async fn prove(
    launch: &Launch,
    child: &mut Owned,
    reader: Readiness,
) -> Result<NodeClient, LocalNodeError> {
    let proof = reader.read(child).await?;
    let identity = launch
        .home()
        .identity()
        .await?
        .ok_or(LocalNodeError::MissingIdentity)?;
    if identity.node != proof.ready.node() {
        return Err(LocalNodeError::Identity {
            expected: identity.node,
            found: proof.ready.node(),
        });
    }
    #[cfg(windows)]
    let connection = proof.connection;
    #[cfg(unix)]
    let connection = {
        let opened = NodeClient::open_local(identity.endpoint.path())
            .await
            .map_err(LocalNodeError::Connect)?;
        // Before ANY token or reap: this retained socket must name this owner.
        child.qualify_peer(opened.peer_process().map_err(LocalNodeError::Connect)?)?;
        opened
    };
    let client = expected_node(
        connection
            .authenticate(&identity.credential, launch.client().clone())
            .await
            .map_err(LocalNodeError::Connect)?,
        identity.node,
    )?;
    tokio::select! {
        biased;
        exit = child.wait() => return Err(LocalNodeError::StartupExit(exit?)),
        result = serving(&client) => result?,
    }
    Ok(client)
}

#[cfg(all(test, unix))]
mod tests;
