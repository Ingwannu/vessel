//! No accept loop: creating another instance would permit challenge relaying.

use std::time::Duration;
use tokio::io::AsyncReadExt as _;
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio::sync::oneshot;
use vessel_proto::NodeId;
use vessel_proto::local_launch::{
    BootstrapChallenge, BootstrapPipe, BootstrapReady, CHALLENGE_RECORD_BYTES,
};

use super::DesktopBootstrapError as Error;
use crate::ipc::LocalGateway;

const PROOF_DEADLINE: Duration = Duration::from_secs(10);
const PRIVATE_INSTANCES: u32 = 1;
pub(crate) type Proof = oneshot::Receiver<Result<BootstrapReady, Error>>;

pub(crate) fn bind(
    name: Option<BootstrapPipe>,
) -> Result<Option<NamedPipeServer>, crate::NodeError> {
    name.map(|name| {
        ServerOptions::new()
            .access_inbound(true)
            .access_outbound(true)
            .first_pipe_instance(true)
            .reject_remote_clients(true)
            .max_instances(PRIVATE_INSTANCES)
            .create(name.path())
            .map_err(Error::Io)
            .map_err(crate::NodeError::DesktopBootstrap)
    })
    .transpose()
}

pub(crate) fn start(
    stream: Option<NamedPipeServer>,
    gateway: LocalGateway,
    node: NodeId,
) -> Option<Proof> {
    let mut stream = stream?;
    let tracker = gateway.connections.clone();
    let (proof, receiver) = oneshot::channel();
    drop(tracker.spawn(async move {
        let challenge = tokio::select! {
            biased;
            () = gateway.agents.closing.cancelled() => Err(Error::Stopped),
            result = tokio::time::timeout(PROOF_DEADLINE, receive(&mut stream)) => {
                result.map_err(Error::Deadline).and_then(std::convert::identity)
            }
        };
        match challenge {
            Ok(challenge) => {
                // Only the binary's inherited stdout receives this result.
                // A vanished receiver closes this instance before any auth.
                if proof
                    .send(Ok(BootstrapReady::new(node, challenge)))
                    .is_err()
                {
                    return;
                }
                // Move the SAME server handle into canonical authentication;
                // never recreate the instance or open the canonical endpoint.
                crate::ipc::connection(stream, gateway).await;
            }
            Err(error) => {
                let _delivered = proof.send(Err(error));
            }
        }
    }));
    Some(receiver)
}

async fn receive(stream: &mut NamedPipeServer) -> Result<BootstrapChallenge, Error> {
    stream.connect().await?;
    let mut record = [0; CHALLENGE_RECORD_BYTES];
    let _read = stream.read_exact(&mut record).await?;
    Ok(BootstrapChallenge::decode(std::str::from_utf8(&record)?)?)
}
