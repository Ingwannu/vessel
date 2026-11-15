//! One responsibility of the Codex session actor, using its private typed state.

use super::{
    AgentEvent, AgentOutput, Command, ContentBlock, InputError, Mode, Pending, Reply, Session,
    SteerOutcome, SteerReply, TurnInterruptParams, TurnStartParams, TurnState, TurnSteerParams,
    answer, mapper,
};

impl Session {
    pub(super) async fn command(&mut self, command: Command) {
        match command {
            Command::NativeCommandOffers { reply } => {
                let _closed = reply.send(crate::NativeCommandOffers::None);
            }
            Command::DecidePlan { request, reply } => {
                crate::input::plan::unsupported(request, reply);
            }
            Command::Native { intent, reply } => {
                crate::input::native_command::unsupported(intent, reply);
            }
            // A side question's context is not carried yet (A2 carries it
            // in its next commit): refused before anything is sent, never
            // dropped (ADR 0057, N5's integration).
            Command::Prompt {
                context: Some(_),
                reply,
                ..
            } => answer(reply, Err(InputError::ContextNotTaken)),
            Command::Prompt {
                content,
                client_message_id,
                reply,
                ..
            } => self.prompt(content, client_message_id, reply).await,
            Command::CancelTurn { reply } => {
                self.cancel().await;
                answer(reply, Ok(()));
            }
            Command::Steer {
                content,
                client_message_id,
                reply,
                ..
            } => self.steer(content, client_message_id, reply).await,
            Command::Answer { id, outcome, reply } => {
                let result = match self.approvals.answer(id, &outcome) {
                    Ok(settled) => {
                        if self.out.answer(&settled.request, &settled.response).await {
                            self.emit(AgentEvent::PermissionResolved { id, outcome })
                                .await;
                            Ok(())
                        } else {
                            Err(InputError::Closed)
                        }
                    }
                    Err(error) => Err(error),
                };
                answer(reply, result);
            }
            Command::AnswerQuestion {
                id,
                answer: given,
                reply,
            } => {
                let result = match self.questions.answer(id, given) {
                    Ok((outcome, settled)) => {
                        if self.out.answer(&settled.request, &settled.response).await {
                            self.emit(AgentEvent::QuestionResolved { id, outcome })
                                .await;
                            Ok(())
                        } else {
                            Err(InputError::Closed)
                        }
                    }
                    Err(error) => Err(error),
                };
                answer(reply, result);
            }
            Command::TurnRunning { reply } => {
                let _gone = reply.send(self.turn != TurnState::Idle);
            }
            Command::Options { reply } => {
                let _gone = reply.send(vec![self.mode.option()]);
            }
            Command::SetOption { choice, reply } => {
                // Kept here, sent with the next `turn/start`.
                let result = match Mode::chosen(&choice) {
                    Ok(mode) => {
                        if mode != self.mode {
                            self.mode = mode;
                            self.output(AgentOutput::Options(vec![mode.option()])).await;
                        }
                        Ok(vec![mode.option()])
                    }
                    Err(error) => Err(InputError::Option(error)),
                };
                let _gone = reply.send(result);
            }
        }
    }

    pub(super) async fn prompt(
        &mut self,
        content: Vec<ContentBlock>,
        client_message_id: Option<vessel_proto::timeline::ClientMessageId>,
        reply: Reply,
    ) {
        if self.turn != TurnState::Idle {
            answer(reply, Err(InputError::TurnInProgress));
            return;
        }
        let Some(thread) = self.thread.clone() else {
            answer(reply, Err(InputError::Closed));
            return;
        };
        let input = match mapper::user_input(&content) {
            Ok(input) => input,
            Err(error) => {
                answer(reply, Err(error));
                return;
            }
        };
        let params = TurnStartParams {
            thread_id: thread,
            input,
            approval_policy: self.mode.approval(),
            approvals_reviewer: self.mode.reviewer(),
            sandbox_policy: self.mode.sandbox_policy(),
        };
        if !self
            .request("turn/start", &params, Pending::TurnStart)
            .await
        {
            answer(reply, Err(InputError::Closed));
            return;
        }
        self.turn = TurnState::Starting { cancel: false };
        self.streamed.clear();
        self.announced.clear();
        self.cancelling = false;
        self.before = self.total;
        self.emit(AgentEvent::TurnStarted {
            content,
            client_message_id,
        })
        .await;
        answer(reply, Ok(()));
    }

    /// Cancel the turn: what waits on a person is withdrawn first (a
    /// command's `cancel` stops the turn too), then the turn interrupted.
    pub(super) async fn cancel(&mut self) {
        if self.turn == TurnState::Idle {
            return;
        }
        self.cancelling = true;
        self.withdraw().await;
        match &mut self.turn {
            TurnState::Idle => {}
            TurnState::Starting { cancel } => *cancel = true,
            TurnState::Running { .. } => self.interrupt().await,
        }
    }

    pub(super) async fn interrupt(&mut self) {
        let (Some(thread), TurnState::Running { id }) = (self.thread.clone(), &self.turn) else {
            return;
        };
        let params = TurnInterruptParams {
            thread_id: thread,
            turn_id: id.clone(),
        };
        let _sent = self
            .request("turn/interrupt", &params, Pending::Interrupt)
            .await;
    }

    pub(super) async fn steer(
        &mut self,
        content: Vec<ContentBlock>,
        client_message_id: Option<vessel_proto::timeline::ClientMessageId>,
        reply: SteerReply,
    ) {
        let (thread, turn) = match (&self.thread, &self.turn) {
            (_, TurnState::Idle) => {
                let _gone = reply.send(Ok(SteerOutcome::PromptRequired));
                return;
            }
            (_, TurnState::Starting { .. }) => {
                let _gone = reply.send(Err(InputError::Steering(
                    "the turn has not started yet; send it again".to_owned(),
                )));
                return;
            }
            (None, TurnState::Running { .. }) => {
                let _gone = reply.send(Err(InputError::Closed));
                return;
            }
            (Some(thread), TurnState::Running { id }) => (thread.clone(), id.clone()),
        };
        let input = match mapper::user_input(&content) {
            Ok(input) => input,
            Err(error) => {
                let _gone = reply.send(Err(error));
                return;
            }
        };
        let params = TurnSteerParams {
            thread_id: thread,
            expected_turn_id: turn,
            input,
        };
        let id = self.next_id();
        if self.out.request(id, "turn/steer", &params).await {
            let _none = self.pending.insert(
                id,
                Pending::Steer {
                    content,
                    client_message_id,
                    reply,
                },
            );
        } else {
            let _gone = reply.send(Err(InputError::Closed));
        }
    }
}
