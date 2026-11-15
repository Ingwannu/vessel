//! Bounded node calls. Every answer echoes the machine and request id,
//! even refusal before a worker starts; the UI can discard stale answers.

use std::sync::Arc;
use tokio::sync::{Semaphore, mpsc};
use vessel_app_view::agent_status::install::{Cursor, ReadNext, after_read};
use vessel_client::NodeClient;
use vessel_proto::agents::{GetAgentStatus, ListAgents, ReadInstall};
use vessel_proto::{ProtocolVersion, RequestId};

use super::{Administration, AgentsError, Answer, Ask, Inventory, MachineKey};
use crate::Event;

pub(crate) struct Hub(Arc<Semaphore>);
impl Default for Hub {
    fn default() -> Self {
        Self(Arc::new(Semaphore::new(4)))
    }
}

impl Hub {
    pub(crate) async fn ask(
        &self,
        client: Option<NodeClient>,
        machine: MachineKey,
        request: RequestId,
        ask: Ask,
        events: mpsc::Sender<Event>,
    ) {
        let Some(client) = client else {
            emit(&events, &machine, request, Err(AgentsError::Disconnected)).await;
            return;
        };
        let Ok(permit) = Arc::clone(&self.0).try_acquire_owned() else {
            emit(&events, &machine, request, Err(AgentsError::Busy)).await;
            return;
        };
        drop(tokio::spawn(async move {
            let _permit = permit;
            if let Err(error) = run(&client, &machine, request, ask, &events).await {
                emit(&events, &machine, request, Err(error)).await;
            }
        }));
    }
}

async fn emit(
    events: &mpsc::Sender<Event>,
    machine: &MachineKey,
    request: RequestId,
    answer: Result<Answer, AgentsError>,
) {
    let _closed = events
        .send(Event::Agents {
            machine: machine.clone(),
            request,
            answer,
        })
        .await;
}

async fn run(
    client: &NodeClient,
    machine: &MachineKey,
    request: RequestId,
    ask: Ask,
    events: &mpsc::Sender<Event>,
) -> Result<(), AgentsError> {
    let answer = match ask {
        Ask::List => {
            let harnesses = client.harnesses().await?;
            let (agents, readiness, administration) =
                if client.protocol() >= ProtocolVersion::new(11) {
                    (
                        client.call(ListAgents {}).await?,
                        client.call(GetAgentStatus::default()).await?,
                        Administration::Supported,
                    )
                } else {
                    (Vec::new(), Vec::new(), Administration::OlderNode)
                };
            Answer::List(Inventory {
                node: client.node().id,
                command_shell: vessel_app_view::agent_status::command::CommandShell::for_os(
                    &client.node().os,
                ),
                harnesses,
                agents,
                readiness,
                administration,
            })
        }
        Ask::Check(agent) => Answer::Checked(
            client
                .call(GetAgentStatus {
                    agent: Some(agent),
                    profile: None,
                    refresh: true,
                })
                .await?,
        ),
        Ask::Install(install) => {
            follow_install(client, machine, request, install, events).await?;
            return Ok(());
        }
    };
    emit(events, machine, request, Ok(answer)).await;
    Ok(())
}

async fn follow_install(
    client: &NodeClient,
    machine: &MachineKey,
    request: RequestId,
    install: super::InstallAgent,
    events: &mpsc::Sender<Event>,
) -> Result<(), AgentsError> {
    let job = client.call(install).await?;
    let id = job.id;
    emit(events, machine, request, Ok(Answer::Installing(job))).await;
    let mut from = Cursor::START;
    let mut wait = Some(30);
    loop {
        let output = client
            .call(ReadInstall {
                job: id,
                from: from.get(),
                wait,
            })
            .await?;
        let next = after_read(from, &output)?;
        if events
            .send(Event::Agents {
                machine: machine.clone(),
                request,
                answer: Ok(Answer::Output(output)),
            })
            .await
            .is_err()
        {
            return Ok(());
        }
        match next {
            ReadNext::Wait(next) => {
                from = next;
                wait = Some(30);
            }
            ReadNext::Drain(next) => {
                from = next;
                wait = None;
            }
            ReadNext::Finished(completion) => {
                emit(events, machine, request, Ok(Answer::Finished(completion))).await;
                return Ok(());
            }
        }
    }
}
