//! Bind exit observations to the authenticated node and its local identity.
//! A loopback address alone must never authorize waiting on another node's lock.

use vessel_client::NodeClient;
use vessel_client::book::Resolved;
use vessel_node::DataDir;
use vessel_proto::StopWhen;

use crate::args::Target;
use crate::connect;
use crate::error::CliError;

use super::local_read::{Kind, Pending};
use super::lock_wait::Released;

/// Reporting keeps a requested shutdown distinct from observed local completion.
pub(super) enum ExitObservation {
    /// A drain request deliberately returns before live sessions finish.
    Draining,
    /// A remote endpoint supplies no local process-exit evidence.
    Requested,
    /// A connection closed without authority over the selected data directory.
    ConnectionClosed,
    /// The matching directory's lock became available and was released again.
    LockReleased,
}

impl ExitObservation {
    /// Human output must state only the postcondition the command observed.
    #[must_use]
    pub(super) const fn message(&self) -> &'static str {
        match self {
            Self::Draining => "the node is draining; it stops when the last live session ends\n",
            Self::Requested => "shutdown requested; remote process exit was not verified\n",
            Self::ConnectionClosed => "the node closed its shutdown connection\n",
            Self::LockReleased => "the node closed its shutdown connection; lock released\n",
        }
    }
}

enum Locality {
    Remote,
    Connection,
    Directory(VerifiedDirectory),
}

struct VerifiedDirectory {
    dir: DataDir,
    released: Option<Released>,
}

/// Keep the original connection with the directory it authenticated as.
/// Private fields prevent substituting another client between bind and wait.
pub(super) struct Observer {
    client: NodeClient,
    locality: Locality,
    when: StopWhen,
}

impl Observer {
    /// Validate local identity before a shutdown request changes anything.
    ///
    /// # Errors
    /// The local identity file cannot be read or decoded.
    pub(super) async fn bind(
        target: &Target,
        resolved: &Resolved,
        client: &NodeClient,
        when: StopWhen,
    ) -> Result<Self, CliError> {
        let locality = locality(target, resolved, client).await?;
        // NodeClient shares its connection; this copies no token or node data.
        Ok(Self {
            client: client.clone(),
            locality,
            when,
        })
    }

    /// Disk observations require the directory authenticated by this connection.
    pub(super) const fn directory(&self) -> Option<&DataDir> {
        match &self.locality {
            Locality::Directory(local) => Some(&local.dir),
            Locality::Remote | Locality::Connection => None,
        }
    }

    /// After request success, wait on the chosen lifecycle event, never a timer.
    ///
    /// # Errors
    /// The OS-lock waiter cannot be created or its I/O fails.
    pub(super) async fn completed(&mut self) -> Result<ExitObservation, CliError> {
        match self.when {
            StopWhen::WhenIdle => return Ok(ExitObservation::Draining),
            StopWhen::Now => {}
        }
        match &mut self.locality {
            Locality::Remote => Ok(ExitObservation::Requested),
            Locality::Connection => {
                self.client.closed().await;
                Ok(ExitObservation::ConnectionClosed)
            }
            Locality::Directory(local) => {
                self.client.closed().await;
                local.event()?.wait().await?;
                Ok(ExitObservation::LockReleased)
            }
        }
    }
}

async fn locality(
    target: &Target,
    resolved: &Resolved,
    client: &NodeClient,
) -> Result<Locality, CliError> {
    if !connect::is_here(resolved) {
        return Ok(Locality::Remote);
    }
    let dir = connect::data_dir(target)?;
    let reading = dir.clone();
    let mut pending = Pending::spawn(
        Kind::Identity,
        move || reading.read_node_id(),
        || tracing::debug!("node identity read completed after its command ended"),
    )?;
    let id = pending
        .wait()
        .await
        .map_err(CliError::IdentityReader)?
        .map_err(CliError::LocalIdentity)?;
    if id == Some(client.node().id) {
        Ok(Locality::Directory(VerifiedDirectory {
            dir,
            released: None,
        }))
    } else {
        Ok(Locality::Connection)
    }
}

impl VerifiedDirectory {
    fn event(&mut self) -> Result<&mut Released, CliError> {
        let event = match self.released.take() {
            Some(event) => event,
            None => Released::subscribe(&self.dir)?,
        };
        Ok(self.released.insert(event))
    }
}
