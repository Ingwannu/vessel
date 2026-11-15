//! One responsibility of the Codex session actor, using its private typed state.

use super::{
    AgentEvent, AgentOutput, Asking, CommandApproval, ContentBlock, DeltaNotification,
    ErrorNotification, FileChangeApproval, INVALID_PARAMS, ItemNotification, METHOD_NOT_FOUND,
    PermissionsApproval, PlanNotification, QuestionOutcome, RequestResolved, Session, Settled,
    ToolCallUpdate, ToolKind, TurnNotification, TurnState, UsageNotification, UserInputRequest,
    Value, asks, mapper, read,
};
use vessel_proto::PermissionOutcome;

impl Session {
    pub(super) async fn notification(&mut self, method: &str, params: Value) {
        let Some(thread) = self.thread.clone() else {
            return;
        };
        for event in self.children.observe(&thread, method, &params) {
            self.emit(event).await;
        }
        let ours = |of: &str| of == thread;
        match method {
            "turn/completed" => self.completed(&thread, params).await,
            // Any thread's: what is asked is asked whichever thread asks.
            "serverRequest/resolved" => {
                if let Some(resolved) = read::<RequestResolved>(method, params) {
                    self.resolved(&resolved.request_id).await;
                }
            }
            "item/started" | "item/completed" => {
                if let Some(item) = read::<ItemNotification>(method, params)
                    .filter(|notification| ours(&notification.thread_id))
                {
                    self.item(method == "item/started", item).await;
                }
            }
            "item/agentMessage/delta" | "item/reasoning/summaryTextDelta" => {
                if let Some(delta) = read::<DeltaNotification>(method, params)
                    .filter(|notification| ours(&notification.thread_id))
                {
                    let content = ContentBlock::Text { text: delta.delta };
                    let _new = self.streamed.insert(delta.item_id);
                    self.emit(if method == "item/agentMessage/delta" {
                        AgentEvent::MessageChunk { content }
                    } else {
                        AgentEvent::ThoughtChunk { content }
                    })
                    .await;
                }
            }
            "turn/plan/updated" => {
                if let Some(plan) = read::<PlanNotification>(method, params)
                    .filter(|notification| ours(&notification.thread_id))
                {
                    self.emit(mapper::plan(&plan.plan)).await;
                }
            }
            "thread/tokenUsage/updated" => {
                if let Some(usage) = read::<UsageNotification>(method, params)
                    .filter(|notification| ours(&notification.thread_id))
                {
                    self.total = usage.token_usage.total;
                    let context = mapper::context(&usage.token_usage);
                    if context.is_some() && context != self.context {
                        self.context.clone_from(&context);
                        if let Some(context) = context {
                            self.output(AgentOutput::Usage(context)).await;
                        }
                    }
                }
            }
            "error" => {
                if let Some(error) = read::<ErrorNotification>(method, params)
                    .filter(|notification| ours(&notification.thread_id))
                {
                    let line = if error.will_retry {
                        format!("Codex will retry: {}", error.error.message)
                    } else {
                        format!("Codex: {}", error.error.message)
                    };
                    self.emit(AgentEvent::HarnessLog { line }).await;
                }
            }
            // `turn/started` among them: the turn's id is taken from
            // `turn/start`'s answer alone, since another client may start a
            // turn on the same thread and its `turn/started` must not become
            // this session's.
            _ => {}
        }
    }

    pub(super) async fn item(&mut self, started: bool, notification: ItemNotification) {
        let item = notification.item;
        if started {
            if let Some(call) = mapper::tool_call(&item) {
                if self.announced.insert(call.id.as_str().to_owned()) {
                    self.emit(AgentEvent::ToolCall(call)).await;
                } else {
                    // Announced from its approval: the item says the rest.
                    let update = ToolCallUpdate {
                        id: call.id,
                        title: Some(call.title),
                        kind: Some(call.kind),
                        status: Some(call.status),
                        content: None,
                        locations: Some(call.locations),
                        raw_input: call.raw_input,
                        raw_output: None,
                    };
                    self.emit(AgentEvent::ToolCallUpdate(update)).await;
                }
            }
            return;
        }
        if let Some(update) = mapper::tool_call_done(&item) {
            self.emit(AgentEvent::ToolCallUpdate(update)).await;
        }
        if let Some((id, whole)) = mapper::whole_text(&item)
            && !self.streamed.contains(id)
        {
            self.emit(whole).await;
        }
    }

    /// `turn/completed` on `thread`: the running turn's ends it; one that
    /// does not read ends the running turn failed, rather than leave the
    /// session busy for good.
    pub(super) async fn completed(&mut self, thread: &str, params: Value) {
        let TurnState::Running { id } = &self.turn else {
            // None running, or its id not known yet: `turn/start` answers
            // before Codex says anything of the turn, so this is another's.
            return;
        };
        let running = id.clone();
        let failed = match serde_json::from_value::<TurnNotification>(params) {
            Ok(done) if done.thread_id != thread || done.turn.id != running => return,
            Ok(done) => {
                let usage = mapper::turn_usage(self.before, self.total);
                mapper::turn_end(&done.turn, usage)
            }
            Err(error) => AgentEvent::TurnFailed {
                message: format!("Codex's turn/completed did not read: {error}"),
            },
        };
        self.turn = TurnState::Idle;
        self.streamed.clear();
        self.withdraw().await;
        self.emit(failed).await;
    }

    /// A request of the agent's: an approval or a question is held for a
    /// person; anything else is refused.
    pub(super) async fn asked(&mut self, id: Value, method: &str, params: Value) {
        match method {
            "item/commandExecution/requestApproval" => {
                let Some(asked) = self.readable::<CommandApproval>(&id, method, params).await
                else {
                    return;
                };
                let title = asked
                    .command
                    .clone()
                    .unwrap_or_else(|| "Run a command".to_owned());
                let input = serde_json::json!({ "command": asked.command, "cwd": asked.cwd });
                let call = asks::tool_call(&asked.item_id, title, ToolKind::Execute, Some(&input));
                self.approval(id, call, Asking::command(&asked)).await;
            }
            "item/fileChange/requestApproval" => {
                let Some(asked) = self
                    .readable::<FileChangeApproval>(&id, method, params)
                    .await
                else {
                    return;
                };
                let title = asked.reason.unwrap_or_else(|| "Edit files".to_owned());
                let call = asks::tool_call(&asked.item_id, title, ToolKind::Edit, None);
                self.approval(id, call, Asking::file_change()).await;
            }
            "item/permissions/requestApproval" => {
                let Some(asked) = self
                    .readable::<PermissionsApproval>(&id, method, params)
                    .await
                else {
                    return;
                };
                let title = asked
                    .reason
                    .clone()
                    .unwrap_or_else(|| "More permissions".to_owned());
                let call = asks::tool_call(
                    &asked.item_id,
                    title,
                    ToolKind::Other,
                    Some(&asked.permissions),
                );
                self.approval(id, call, Asking::permissions(&asked)).await;
            }
            "item/tool/requestUserInput" => self.user_input(id, method, params).await,
            "mcpServer/elicitation/request" => self.elicitation(id, &params).await,
            _ => {
                let message = format!("Vessel does not serve {method}");
                let _sent = self.out.refuse(&id, METHOD_NOT_FOUND, &message).await;
                tracing::info!(method, "a Codex request Vessel does not serve, refused");
            }
        }
    }

    /// A request's parameters as `T`; one that does not read is refused,
    /// with a line in the log, so Codex never waits on it.
    pub(super) async fn readable<T: serde::de::DeserializeOwned>(
        &self,
        id: &Value,
        method: &str,
        params: Value,
    ) -> Option<T> {
        match serde_json::from_value(params) {
            Ok(read) => Some(read),
            Err(error) => {
                let message = format!("Vessel could not read {method}: {error}");
                let _sent = self.out.refuse(id, INVALID_PARAMS, &message).await;
                self.emit(AgentEvent::HarnessLog {
                    line: format!("vessel: {message}; refused"),
                })
                .await;
                None
            }
        }
    }

    /// Hold an approval on `call`, announcing the call if Codex has not.
    pub(super) async fn approval(
        &mut self,
        request: Value,
        call: vessel_proto::ToolCall,
        asking: Asking,
    ) {
        let tool_call = call.id.clone();
        if self.announced.insert(call.id.as_str().to_owned()) {
            self.emit(AgentEvent::ToolCall(call)).await;
        }
        let (asked, displaced) = self.approvals.open(request, tool_call, asking);
        if let Some((oldest, settled)) = displaced {
            self.settle(&settled).await;
            self.emit(AgentEvent::PermissionResolved {
                id: oldest,
                outcome: PermissionOutcome::Cancelled,
            })
            .await;
        }
        self.emit(AgentEvent::PermissionRequested(asked)).await;
        if self.cancelling {
            // Asked before Codex saw the cancel: withdrawn, as the ACP
            // driver withdraws one; the log shows it asked and withdrawn.
            self.withdraw().await;
        }
    }

    /// Codex's own questions.
    pub(super) async fn user_input(&mut self, id: Value, method: &str, params: Value) {
        if !self.setup.questions {
            let _sent = self.out.answer(&id, &asks::no_answers()).await;
            self.log(
                "Codex asked a question; nobody watches this session, so it was left unanswered",
            )
            .await;
            return;
        }
        let Some(asked) = self.readable::<UserInputRequest>(&id, method, params).await else {
            return;
        };
        match self.questions.user_input(id.clone(), &asked) {
            Ok(opened) => self.question(opened).await,
            Err(why) => {
                let _sent = self.out.answer(&id, &asks::no_answers()).await;
                self.log(&format!(
                    "Codex asked a question Vessel cannot show ({why}); left unanswered"
                ))
                .await;
            }
        }
    }

    /// A tool server's form.
    pub(super) async fn elicitation(&mut self, id: Value, params: &Value) {
        let declined = asks::elicitation_response(&QuestionOutcome::Declined);
        if !self.setup.questions {
            let _sent = self.out.answer(&id, &declined).await;
            self.log(
                "a tool server asked a question; nobody watches this session, so it was declined",
            )
            .await;
            return;
        }
        let thread = self.thread.clone().unwrap_or_default();
        match self.questions.elicitation(id.clone(), &thread, params) {
            Ok(opened) => self.question(opened).await,
            Err(why) => {
                let _sent = self.out.answer(&id, &declined).await;
                self.log(&format!(
                    "a tool server asked a question Vessel cannot show ({why}); declined"
                ))
                .await;
            }
        }
    }

    /// Publish a question held, after the one it displaced.
    pub(super) async fn question(&mut self, (question, displaced): asks::Opened) {
        if let Some((oldest, settled)) = displaced {
            self.settle(&settled).await;
            self.emit(AgentEvent::QuestionResolved {
                id: oldest,
                outcome: QuestionOutcome::Cancelled,
            })
            .await;
        }
        self.emit(AgentEvent::QuestionAsked(question)).await;
        if self.cancelling {
            self.withdraw().await;
        }
    }

    /// Codex no longer waits on request `request`.
    pub(super) async fn resolved(&mut self, request: &Value) {
        if let Some(id) = self.approvals.resolved(request) {
            self.emit(AgentEvent::PermissionResolved {
                id,
                outcome: PermissionOutcome::Cancelled,
            })
            .await;
        }
        if let Some(id) = self.questions.resolved(request) {
            self.emit(AgentEvent::QuestionResolved {
                id,
                outcome: QuestionOutcome::Cancelled,
            })
            .await;
        }
    }

    /// Withdraw what waits on a person, telling Codex and the session.
    pub(super) async fn withdraw(&mut self) {
        for (id, settled) in self.approvals.withdraw_all() {
            self.settle(&settled).await;
            self.emit(AgentEvent::PermissionResolved {
                id,
                outcome: PermissionOutcome::Cancelled,
            })
            .await;
        }
        for (id, settled) in self.questions.withdraw_all() {
            self.settle(&settled).await;
            self.emit(AgentEvent::QuestionResolved {
                id,
                outcome: QuestionOutcome::Cancelled,
            })
            .await;
        }
    }

    /// Answer Codex's request as settled.
    pub(super) async fn settle(&self, settled: &Settled) {
        if !self.out.answer(&settled.request, &settled.response).await {
            tracing::debug!("an answer to Codex not sent: its input is closed");
        }
    }

    pub(super) async fn log(&self, what: &str) {
        tracing::info!(what, "a Codex question not asked");
        self.emit(AgentEvent::HarnessLog {
            line: format!("vessel: {what}"),
        })
        .await;
    }
}
