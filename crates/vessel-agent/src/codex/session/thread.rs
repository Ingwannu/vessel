//! Thread start and resume share the native developer-instruction channel.

use serde_json::Value;
use vessel_proto::settings::AppendSystemPrompt;
use vessel_proto::{AgentAbilities, AgentEvent, HarnessSessionId};

use super::{Pending, Session, handshake_failed, version_of};
use crate::codex::mode::Mode;
use crate::codex::protocol::{
    InitializeResponse, ThreadOpened, ThreadResumeParams, ThreadStartParams,
};
use crate::codex::wire::RpcError;
use crate::error::Disconnect;
use crate::handshake::ForkRefusal;
use crate::introduction::Introduction;
use crate::output::AgentOutput;

impl Session {
    /// `initialize` answered: say who the agent is, then open the thread.
    pub(super) async fn initialized(
        &mut self,
        answer: Result<Value, RpcError>,
    ) -> Option<Disconnect> {
        let response: InitializeResponse = match answer.map(serde_json::from_value) {
            Ok(Ok(response)) => response,
            Ok(Err(error)) => return Some(handshake_failed(format!("initialize: {error}"))),
            Err(error) => return Some(handshake_failed(format!("initialize: {}", error.message))),
        };
        let introduction = Introduction {
            version: response.user_agent.as_deref().and_then(version_of),
            methods: Vec::new(),
        };
        self.output(AgentOutput::Introduced(Box::new(introduction)))
            .await;
        if !self.out.notify("initialized", &serde_json::json!({})).await {
            return Some(Disconnect::Closed);
        }
        // The mode is the one option until the options slice: another
        // choice fails the handshake, as an ACP agent's unknown option
        // does, rather than run without it unannounced.
        for choice in &self.setup.options {
            match Mode::chosen(choice) {
                Ok(mode) => self.mode = mode,
                Err(error) => return Some(handshake_failed(error.to_string())),
            }
        }
        if self.setup.fork.is_some() {
            // Forking a thread is the history slice's (`thread/fork`).
            return Some(Disconnect::Handshake {
                message: "this driver does not fork Codex threads yet".to_owned(),
                refused: None,
                fork_refused: Some(ForkRefusal::CannotFork),
            });
        }
        let cwd = self.setup.cwd.display().to_string();
        let model = self.setup.model.as_ref().map(ToString::to_string);
        let prompt = self.launched.append_system_prompt.clone();
        let developer_instructions = prompt.as_ref().map(AppendSystemPrompt::effective);
        let sent = if let Some(thread) = &self.setup.resume {
            let params = ThreadResumeParams {
                thread_id: thread.to_string(),
                cwd,
                model,
                // Explicit blank clears a supplement on an existing thread.
                developer_instructions,
                approval_policy: self.mode.approval(),
                approvals_reviewer: self.mode.reviewer(),
                sandbox: self.mode.sandbox(),
                exclude_turns: true,
            };
            self.request("thread/resume", &params, Pending::Thread)
                .await
        } else {
            let params = ThreadStartParams {
                cwd,
                model,
                developer_instructions: developer_instructions.filter(|value| !value.is_empty()),
                approval_policy: self.mode.approval(),
                approvals_reviewer: self.mode.reviewer(),
                sandbox: self.mode.sandbox(),
            };
            self.request("thread/start", &params, Pending::Thread).await
        };
        (!sent).then_some(Disconnect::Closed)
    }

    /// The thread is open: the session is ready.
    pub(super) async fn opened(&mut self, answer: Result<Value, RpcError>) -> Option<Disconnect> {
        let opened: ThreadOpened = match answer.map(serde_json::from_value) {
            Ok(Ok(opened)) => opened,
            Ok(Err(_)) | Err(_) if self.launched.append_system_prompt.is_some() => {
                return Some(handshake_failed("Codex did not open the thread".to_owned()));
            }
            Ok(Err(error)) => return Some(handshake_failed(format!("the thread: {error}"))),
            Err(error) => return Some(handshake_failed(error.message)),
        };
        let abilities = AgentAbilities {
            steer: true,
            resume: true,
            image: false,
            audio: false,
            fork: false,
        };
        self.thread = Some(opened.thread.id.clone());
        // The mode alone until slice 3 (`model/list`).
        self.output(AgentOutput::Options(vec![self.mode.option()]))
            .await;
        self.emit(AgentEvent::Ready {
            harness_session: HarnessSessionId::new(opened.thread.id),
            abilities: Some(abilities),
        })
        .await;
        // Nor tool servers (the thread's `config`, slice 3): said, so a
        // session never lacks them unannounced.
        if !self.setup.mcp_servers.is_empty() {
            let names: Vec<String> = self
                .setup
                .mcp_servers
                .keys()
                .map(ToString::to_string)
                .collect();
            let line = format!(
                "vessel: Codex's own protocol does not pass tool servers yet; this session runs \
                 without {}",
                names.join(", ")
            );
            self.emit(AgentEvent::HarnessLog { line }).await;
        }
        None
    }
}
