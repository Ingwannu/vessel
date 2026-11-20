//! Retain the verified process, debugger and profile together through shutdown.

use std::time::Duration;

use tokio::task::JoinHandle;
use vessel_proc::{Child, OwnedGroupExit, ProcessLifetime, Signal, Spawned};

use super::{CdpError, Client, Events};
use crate::cdp::{requests, transport};
use crate::launch::{LaunchError, Plan};
use crate::preparation::{Preparation, PreparationError};

const PRODUCT: &str = "Chrome/149.0.7827.55";
const CLEANUP_DEADLINE: Duration = Duration::from_secs(6);

#[derive(Debug, thiserror::Error)]
pub(super) enum OwnerError {
    #[error("browser executable or browser bundle could not be attested")]
    Identity(#[from] LaunchError),
    #[error("browser reported a different runtime version")]
    Version,
    #[error("owned browser debugger could not complete its operation")]
    Debugger(#[from] CdpError),
    #[error("could not signal the owned browser process group")]
    Signal(#[from] vessel_proc::SignalError),
    #[error("could not confirm owned browser process cleanup")]
    Group(#[from] vessel_proc::CheckedWaitError),
    #[error("owned browser process did not close within its deadline")]
    ProcessDeadline(#[source] tokio::time::error::Elapsed),
    #[error("owned browser stderr drain did not close within its deadline")]
    StderrDeadline(#[source] tokio::time::error::Elapsed),
    #[error("owned browser stderr worker could not finish")]
    Worker(#[from] tokio::task::JoinError),
    #[error("owned browser stderr could not be drained")]
    Stderr(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Readiness {
    Inspecting,
    Ready,
    Closing,
}

/// A staged process owner. A group receipt alone never releases the profile or
/// advertises engine readiness to the node. An eventual node actor owns this
/// value and keeps it through cancellation of any client waiting for close.
pub(super) struct Owner {
    child: Child,
    plan: Plan,
    client: Client,
    events: Events,
    stderr: JoinHandle<std::io::Result<u64>>,
    state: Readiness,
}

impl Owner {
    pub(super) fn spawn(plan: Plan) -> Result<Self, vessel_proc::SpawnError> {
        let Spawned {
            child,
            stdin,
            stdout,
            mut stderr,
        } = plan.spawn_for_diagnostic()?;
        let (client, events) = Client::start(transport::Pipe::new(stdout, stdin));
        // The child can keep writing after debugger shutdown. A live drain
        // avoids a blocked browser and keeps raw Chromium diagnostics private.
        let stderr = tokio::spawn(async move {
            let mut sink = tokio::io::sink();
            tokio::io::copy(&mut stderr, &mut sink).await
        });
        Ok(Self {
            child,
            plan,
            client,
            events,
            stderr,
            state: Readiness::Inspecting,
        })
    }

    pub(super) fn lifetime(&self) -> ProcessLifetime {
        self.child.lifetime()
    }

    pub(super) async fn attest(
        self,
        preparation: &Preparation,
    ) -> Result<(Self, Result<(), OwnerError>), PreparationError> {
        let Self {
            child,
            plan,
            client,
            events,
            stderr,
            state,
        } = self;
        let reported = client.execute(None, requests::Version {}).await;
        let (child, plan, observed) = preparation
            .run(move || {
                let observed = plan.verify_started(&child);
                (child, plan, observed)
            })
            .await?;
        let readiness = match attest_candidate(observed, reported) {
            Ok(()) => client
                .execute(None, requests::DenyDownloads::new())
                .await
                .map(|_ack| ())
                .map_err(OwnerError::from),
            Err(error) => Err(error),
        };
        let state = if readiness.is_ok() {
            Readiness::Ready
        } else {
            state
        };
        Ok((
            Self {
                child,
                plan,
                client,
                events,
                stderr,
                state,
            },
            readiness,
        ))
    }

    pub(super) fn controller(&mut self) -> Option<(&Client, &mut Events)> {
        (self.state == Readiness::Ready).then_some((&self.client, &mut self.events))
    }

    pub(super) async fn close(&mut self) -> Result<OwnedGroupExit, OwnerError> {
        self.state = Readiness::Closing;
        let close = self.client.execute(None, requests::CloseBrowser {}).await;
        let driver = self.client.shutdown().await;
        if close.is_err() {
            self.child.signal(Signal::Kill)?;
        }
        let process = wait_closed(&mut self.child).await;
        let stderr = tokio::time::timeout(CLEANUP_DEADLINE, &mut self.stderr).await;
        let receipt = process?;
        let worker = stderr.map_err(OwnerError::StderrDeadline)?;
        let drained = worker?;
        let _discarded = drained?;
        let _acknowledged = close?;
        driver?;
        Ok(receipt)
    }
}

fn attest_candidate(
    identity: Result<(), LaunchError>,
    reported: Result<requests::BrowserVersion, CdpError>,
) -> Result<(), OwnerError> {
    identity?;
    if reported?.product.as_str() != PRODUCT {
        return Err(OwnerError::Version);
    }
    Ok(())
}

async fn wait_closed(child: &mut Child) -> Result<OwnedGroupExit, OwnerError> {
    match tokio::time::timeout(CLEANUP_DEADLINE, child.wait_checked()).await {
        Ok(result) => Ok(result?),
        Err(deadline) => {
            child.signal(Signal::Kill)?;
            let _receipt = tokio::time::timeout(CLEANUP_DEADLINE, child.wait_checked())
                .await
                .map_err(OwnerError::ProcessDeadline)??;
            Err(OwnerError::ProcessDeadline(deadline))
        }
    }
}
