//! The phases owned by one boot attempt; no new comparison/session registry.
use super::{Context, NodeService, Run, wire};
use vessel_proto::{ContentBlock, ErrorCode, WireError};
use vessel_session::SessionHandle;
impl NodeService {
    pub(super) async fn recover_turn(
        &self,
        context: &Context<'_>,
        run: &mut Run,
    ) -> Result<(), WireError> {
        let Context {
            candidate,
            attempt: _,
            stop: _,
            deadline: _,
            generation: _,
        } = *context;
        let id = candidate.session;
        let _claim = self.sessions.claim(id).map_err(|error| wire(&error))?;
        *run = Run::NotLaunched;
        let ended = self
            .stage(context, self.sessions.ended(id))
            .await?
            .map_err(|error| wire(&error))?;
        self.refuse_installing(&ended.info.spec.harness)
            .map_err(|error| wire(&error))?;
        let _worktree = match ended.info.spec.place.as_deref() {
            Some(vessel_proto::workspace::SessionPlace::Worktree { worktree }) => {
                Some(self.workspaces.start_in(*worktree).ok_or_else(|| {
                    WireError::new(
                        ErrorCode::Conflict,
                        "the session's worktree is busy; resume it manually later",
                    )
                })?)
            }
            Some(vessel_proto::workspace::SessionPlace::Project { .. }) | None => None,
        };
        let creator = ended.info.created_by.clone();
        let next = ended.info.next_seq;
        let mut launch = self
            .launcher
            .again(&self.profiles, &ended.info)
            .map_err(|error| wire(&error))?;
        self.stage(context, self.schedules.starting_again(id))
            .await?
            .map_err(|_closed| {
                WireError::new(ErrorCode::Unavailable, "the scheduler is stopping")
            })?;
        let grant = self
            .stage(context, self.devices.recovery_grant(&creator))
            .await??;
        drop(grant);
        let launched = self
            .models
            .launch(launch.profile.as_ref(), id)
            .map_err(|error| wire(&error))?;
        if launched.as_ref().map(|(model, _)| &model.route)
            != ended.info.model.as_ref().map(|model| &model.route)
        {
            return Err(WireError::new(
                ErrorCode::Unsupported,
                "the session's model route changed; choose how to resume it manually",
            ));
        }
        launch.model_proxy = launched.map(|(_model, proxy)| proxy);
        let prepared = self
            .stage(context, self.sessions.prepare_again(ended, launch))
            .await?
            .map_err(|error| wire(&error))?;
        let reopening = self.admit_launch(context, prepared, &creator).await?;
        *run = Run::Loading(reopening.created.handle.clone());
        // This mutation is already admitted. Never drop its completion just
        // because Off, stop, a deadline or an RPC caller went away.
        let handle = self
            .sessions
            .adopt_again(
                reopening,
                next,
                candidate.generation,
                vessel_store::ReopenMode::Startup,
            )
            .await
            .map_err(|error| wire(&error))?;
        self.recover_prompt(context, &handle, creator, run).await
    }

    async fn admit_launch(
        &self,
        context: &Context<'_>,
        prepared: vessel_session::PreparedAgain<'_>,
        creator: &vessel_proto::Creator,
    ) -> Result<vessel_session::Reopening, WireError> {
        let Context {
            candidate,
            attempt,
            stop,
            generation,
            ..
        } = *context;
        let id = candidate.session;
        let mut prepared = Some(prepared);
        let reopening = loop {
            let mut changed = self.recovery.policy.watch();
            let grant = self
                .stage(context, self.devices.recovery_grant(creator))
                .await??;
            let mut effect = Some(|_policy: &vessel_proto::restart::RestartRecovery| {
                if stop.is_cancelled() || self.lifecycle.is_draining() {
                    return Err(WireError::new(
                        ErrorCode::Unavailable,
                        "the node stopped before launch admission",
                    ));
                }
                let mut prepared = prepared.take().ok_or_else(|| {
                    WireError::new(ErrorCode::Internal, "startup relaunch was already consumed")
                })?;
                let secret = self
                    .agents
                    .reserve(id, &grant.principal, self.launcher.has_tools())
                    .map_err(|error| wire(&error))?;
                if let Some(secret) = &secret {
                    self.launcher
                        .attach_tools(prepared.mcp_servers(), id, secret)
                        .map_err(|error| wire(&error))?;
                }
                prepared
                    .start()
                    .map_err(|error| WireError::new(error.code(), error.to_string()))
            });
            let admitted = attempt.admit(generation, &mut effect)?;
            drop(grant);
            if let Some(reopening) = admitted {
                break reopening;
            }
            self.stage(context, changed.changed())
                .await?
                .map_err(|_closed| {
                    WireError::new(ErrorCode::Unavailable, "startup recovery policy ended")
                })?;
        };
        Ok(reopening)
    }

    async fn recover_prompt(
        &self,
        context: &Context<'_>,
        handle: &SessionHandle,
        creator: vessel_proto::Creator,
        run: &mut Run,
    ) -> Result<(), WireError> {
        let Context {
            attempt,
            stop,
            generation,
            ..
        } = *context;
        let _ready = self
            .stage(context, crate::sessions::again::ready(handle))
            .await?
            .map_err(|error| wire(&error))?;
        let permit = self
            .stage(context, handle.reserve_prompt())
            .await?
            .map_err(|error| WireError::new(error.code(), error.to_string()))?;
        let mut effect = Some(|policy: &vessel_proto::restart::RestartRecovery| {
            if stop.is_cancelled() || self.lifecycle.is_draining() {
                return Err(WireError::new(
                    ErrorCode::Unavailable,
                    "the node stopped before prompt admission",
                ));
            }
            permit
                .send(
                    vec![ContentBlock::Text {
                        text: policy.prompt.as_str().to_owned(),
                    }],
                    creator.clone(),
                )
                .map_err(|error| WireError::new(error.code(), error.to_string()))
        });
        let answer = loop {
            let mut changed = self.recovery.policy.watch();
            let grant = self
                .stage(context, self.devices.recovery_grant(&creator))
                .await??;
            let admitted = attempt.admit_prompt(generation, handle, &mut effect)?;
            if let Some(answer) = admitted {
                self.watch_session(handle, &grant.principal);
                drop(grant);
                break answer;
            }
            drop(grant);
            self.stage(context, changed.changed())
                .await?
                .map_err(|_closed| {
                    WireError::new(ErrorCode::Unavailable, "startup recovery policy ended")
                })?;
        };
        // Prompt admission ends recovery ownership of policy: a later Off
        // must not kill the already-sent turn. Its actual reply still settles.
        *run = Run::PromptAdmitted(handle.clone());
        let outcome = answer.await.map_err(|_closed| {
            WireError::new(
                ErrorCode::Unavailable,
                "the startup prompt outcome is unknown; it will not be retried",
            )
        })?;
        outcome
            .map(|_delivered| ())
            .map_err(|error| WireError::new(error.code(), error.to_string()))
    }
}
