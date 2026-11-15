//! Codex initialize/thread-open handshake, separated for prompt-free catalog ingress.

use serde_json::Value;
use vessel_proto::{AgentAbilities, AgentEvent, HarnessSessionId};

use super::super::protocol::{
    InitializeResponse, ThreadOpened, ThreadResumeParams, ThreadStartParams,
};
use super::super::wire::RpcError;
use super::{Disconnect, Mode, Pending, Session, handshake_failed, version_of};
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
        self.version_policy =
            super::super::policy::CodexVersionPolicy::observed(introduction.version.as_ref());
        self.mode = Mode::default_for(self.version_policy);
        self.output(AgentOutput::Introduced(Box::new(introduction)))
            .await;
        if !self.out.notify("initialized", &serde_json::json!({})).await {
            return Some(Disconnect::Closed);
        }
        // The mode is the one option until the options slice: another
        // choice fails the handshake, as an ACP agent's unknown option
        // does, rather than run without it unannounced.
        for choice in &self.setup.options {
            match Mode::chosen(choice, self.version_policy) {
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
        let sent = if let Some(thread) = &self.setup.resume {
            let params = ThreadResumeParams {
                config: super::super::mcp::ThreadConfig::from_servers(&self.setup.mcp_servers),
                thread_id: thread.to_string(),
                cwd,
                model,
                approval_policy: self.mode.approval(),
                approvals_reviewer: self.mode.reviewer(),
                sandbox: self.mode.sandbox(),
                exclude_turns: true,
            };
            self.request("thread/resume", &params, Pending::Thread)
                .await
        } else {
            let params = ThreadStartParams {
                config: super::super::mcp::ThreadConfig::from_servers(&self.setup.mcp_servers),
                cwd,
                model,
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
        if !self.request_catalog().await {
            return Some(Disconnect::Closed);
        }
        // The mode alone until slice 3 (`model/list`).
        self.output(AgentOutput::Options(vec![
            self.mode.option(self.version_policy),
        ]))
        .await;
        self.emit(AgentEvent::Ready {
            harness_session: HarnessSessionId::new(opened.thread.id),
            abilities: Some(abilities),
        })
        .await;
        None
    }
}
