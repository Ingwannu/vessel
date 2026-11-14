//! Setting the ACP connection up over the agent's stdio: the SDK's
//! handlers, which only forward what arrives into the loop's `inbound`
//! queue, the loop's state, and what is reported when the connection ends.

use agent_client_protocol::v1::schema as acp;
use agent_client_protocol::{Client, Error};
use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::mpsc;
use vessel_proto::AgentAbilities;

use super::{Driver, INBOUND, Inbound, describe, forward};
use crate::error::Disconnect;
use crate::handshake::AgentSetup;
use crate::input::Command;
use crate::options::Options;
use crate::output::AgentOutput;
use crate::permissions::Permissions;
use crate::questions::Questions;
use crate::recent::Recent;
use crate::transport::{self, Gate};

/// Drive the ACP connection over the agent's stdio until it ends, then
/// report why.
pub(crate) async fn run(
    stdin: ChildStdin,
    stdout: ChildStdout,
    commands: mpsc::Receiver<Command>,
    outputs: mpsc::Sender<AgentOutput>,
    setup: AgentSetup,
) {
    let (inbound, inbound_receiver) = mpsc::channel(INBOUND);
    let questions = Questions::new(setup.questions);
    let gate = Gate::new();
    let transport = transport::lines(stdin, stdout, gate.clone());
    // Outside the connection's closure, so it can still be settled when the
    // connection fails rather than closes (see the end of this function).
    let mut driver = Driver {
        inbound: inbound.clone(),
        inbound_receiver,
        commands,
        commands_open: true,
        outputs: outputs.clone(),
        session: None,
        turn: false,
        cancelling: false,
        permissions: Permissions::default(),
        questions,
        tool_calls: Recent::default(),
        tail: super::tail::Tail::default(),
        gate,
        options: Options::default(),
        reported: None,
        abilities: AgentAbilities::default(),
        last_commands: None,
        last_usage: None,
    };
    let ended = Client
        .v1()
        .name("vessel")
        .on_receive_notification(
            {
                let inbound = inbound.clone();
                async move |notification: acp::SessionNotification, _cx| {
                    forward(&inbound, Inbound::Update(notification)).await;
                    Ok::<(), Error>(())
                }
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .on_receive_request(
            {
                let inbound = inbound.clone();
                async move |request: acp::RequestPermissionRequest, responder, _cx| {
                    forward(&inbound, Inbound::Permission(request, responder)).await;
                    Ok::<(), Error>(())
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: acp::CreateElicitationRequest, responder, _cx| {
                forward(&inbound, Inbound::Question(request, responder)).await;
                Ok::<(), Error>(())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_with(transport, async |cx| driver.run(cx, setup).await)
        .await;
    let disconnect = match ended {
        Ok(disconnect) => disconnect,
        Err(error) => {
            // The transport failed (for example the agent wrote bytes that
            // are not UTF-8): the loop was dropped without seeing a close,
            // so the turn, and the permission requests and questions it
            // left open, are settled here, as on a close.
            driver.abandon_turn().await;
            Disconnect::Protocol {
                message: describe(&error),
            }
        }
    };
    drop(driver);
    if outputs
        .send(AgentOutput::Disconnected(disconnect))
        .await
        .is_err()
    {
        tracing::debug!("agent outputs dropped before the disconnect");
    }
}
