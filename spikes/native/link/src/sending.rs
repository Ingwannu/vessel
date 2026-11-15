//! What the link sends to a session: a conversation started with its
//! first prompt, a prompt (with its files, track P's row c), a session
//! started again, an input. Kept beside `lib.rs` so the link's command loop
//! stays one screen of dispatch (the file's size is ratcheted).

use vessel_proto::{
    ContentBlock, Delivery, HarnessRef, OptionChoice, ProtocolVersion, SessionId, SessionInfo,
    SessionInput, SessionMode, SessionSpec,
};

use vessel_app_view::Card;

use crate::{Event, LinkError, State, attachments};

impl State {
    /// Create a conversation and send its first prompt (none when it
    /// waits for its files, track P's row c): `session.prompt`
    /// where the node speaks protocol 10, `session.send` before it.
    pub(super) async fn start(
        &mut self,
        machine: &str,
        harness: HarnessRef,
        folder: Option<String>,
        prompt: Option<String>,
        options: Vec<OptionChoice>,
    ) {
        let started = async {
            let client = self.client(machine)?.clone();
            let mode = SessionMode::Structured {
                model: None,
                resume: None,
                options,
            };
            let mut spec = SessionSpec::new(harness, mode);
            if let Some(folder) = folder {
                spec = spec.with_cwd(folder);
            }
            let info: SessionInfo = client
                .create_session(spec)
                .await
                .map_err(|error| LinkError::Call(error.to_string()))?;
            if let Some(prompt) = prompt {
                let content = vec![ContentBlock::Text { text: prompt }];
                let sent = if client.protocol() >= ProtocolVersion::new(10) {
                    client
                        .prompt(info.id, content, Delivery::Start)
                        .await
                        .map(|_delivered| ())
                } else {
                    client.send(info.id, SessionInput::Prompt { content }).await
                };
                sent.map_err(|error| LinkError::Call(error.to_string()))?;
            }
            Ok::<_, LinkError>(Card::from_session(machine, &info))
        }
        .await;
        match started {
            Ok(card) => {
                self.emit(Event::Started(card)).await;
                self.refresh().await;
            }
            Err(error) => self.emit(Event::Failed(error.to_string())).await,
        }
    }

    /// What a composer's files ask (track P's row c).
    pub(super) async fn files(&mut self, files: attachments::Files) {
        match files {
            attachments::Files::Suggest(ask) => self.mention(ask),
            attachments::Files::Capabilities(ask) => {
                let client = self
                    .attachment_client(&ask.machine, ask.expected_node)
                    .map_err(attachments::CapabilityFailure::Target);
                attachments::spawn_capabilities(client, ask, self.events.clone());
            }
            attachments::Files::Send(attach) => self.attach(attach),
            attachments::Files::Create(create) => {
                let attachments::Create {
                    machine,
                    harness,
                    folder,
                    options,
                } = create;
                self.start(&machine, harness, folder, None, options).await;
            }
            attachments::Files::Prompt(prompt) => self.prompt_files(prompt).await,
            attachments::Files::Wake(wake) => self.wake_files(wake).await,
            attachments::Files::Read(sent) => {
                let client = self
                    .attachment_client(&sent.machine, sent.node)
                    .map_err(LinkError::AttachmentTarget);
                attachments::spawn_read(client, sent, self.events.clone());
            }
        }
    }

    /// Send a composer's file beside the other commands
    /// (`attachments::spawn`).
    fn attach(&self, attach: attachments::Attach) {
        let client = self
            .attachment_client(&attach.machine, attach.expected_node)
            .map_err(LinkError::AttachmentTarget);
        attachments::spawn(client, attach, self.events.clone());
    }

    /// The attachment prompt follows only once capability records are read.
    async fn wake_files(&mut self, wake: attachments::Wake) {
        let result = async {
            let client = self.attachment_client(&wake.machine, wake.expected_node)?;
            tokio::time::timeout(WAKE_FILES_DEADLINE, client.resume_session(wake.session))
                .await
                .map_err(|_elapsed| LinkError::AttachmentRead(attachments::ReadProblem::TimedOut))?
                .map_err(LinkError::AttachmentCall)
        }
        .await;
        let outcome = match result {
            Ok(_) => attachments::WakeOutcome::Ready,
            Err(why) => {
                tracing::info!(%why, session = %wake.session, "a session could not resume for attachments");
                attachments::WakeOutcome::Refused
            }
        };
        self.emit(Event::Files(attachments::Answer::Woke {
            message: wake.message,
            session: wake.session,
            outcome,
        }))
        .await;
        self.refresh().await;
    }

    /// Start `session` again, then send it `text`, and list the fleet
    /// again (its card is live once more).
    pub(super) async fn resume(&mut self, machine: &str, session: SessionId, text: String) {
        let resumed = async {
            let client = self.client(machine)?.clone();
            let _info = client
                .resume_session(session)
                .await
                .map_err(|error| LinkError::Call(error.to_string()))?;
            let content = vec![ContentBlock::Text { text }];
            client
                .prompt(session, content, Delivery::Queue)
                .await
                .map(|_delivered| ())
                .map_err(|error| LinkError::Call(error.to_string()))
        }
        .await;
        match resumed {
            Ok(()) => {
                self.emit(Event::Resumed { session }).await;
                self.refresh().await;
            }
            Err(error) => {
                self.emit(Event::ResumeFailed {
                    session,
                    why: error.to_string(),
                })
                .await;
            }
        }
    }

    pub(super) async fn prompt(
        &self,
        machine: &str,
        session: SessionId,
        text: String,
        delivery: Delivery,
    ) {
        let content = vec![ContentBlock::Text { text }];
        self.deliver(machine, session, content, delivery, None)
            .await;
    }

    async fn deliver(
        &self,
        machine: &str,
        session: SessionId,
        content: Vec<ContentBlock>,
        delivery: Delivery,
        expected_node: Option<vessel_proto::NodeId>,
    ) {
        let Some(client) = self.clients.get(machine).map(|connected| &connected.client) else {
            let why = LinkError::NotConnected(machine.to_owned()).to_string();
            self.emit(Event::SessionLost { session, why }).await;
            return;
        };
        if let Some(expected) = expected_node {
            let actual = client.node().id;
            if actual != expected {
                self.emit(Event::Failed(
                    LinkError::AttachmentTarget(attachments::TargetProblem::Changed {
                        expected,
                        actual,
                    })
                    .to_string(),
                ))
                .await;
                return;
            }
        }
        match client.prompt(session, content, delivery).await {
            Ok(_delivered) => self.emit(Event::Sent { session }).await,
            Err(error) => {
                self.emit(Event::Failed(
                    LinkError::Call(error.to_string()).to_string(),
                ))
                .await;
            }
        }
    }
    pub(super) async fn input(&self, machine: &str, session: SessionId, input: SessionInput) {
        let Some(client) = self.clients.get(machine).map(|connected| &connected.client) else {
            let why = LinkError::NotConnected(machine.to_owned()).to_string();
            self.emit(Event::SessionLost { session, why }).await;
            return;
        };
        if let Err(error) = client.send(session, input).await {
            self.emit(Event::Failed(
                LinkError::Call(error.to_string()).to_string(),
            ))
            .await;
        }
    }
}

/// A resume that never replies cannot keep the composer waiting forever.
const WAKE_FILES_DEADLINE: std::time::Duration = std::time::Duration::from_secs(30);
