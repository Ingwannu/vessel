//! An activity-only RPC handler has only a `TerminalHandle`, never `NodeService`
//! or a fabricated owner principal. All other methods are rejected through
//! the shared method table, including future methods added by another area.
use vessel_proto::notifications::{ReportTerminalActivity, TerminalAttention};
use vessel_proto::{ErrorCode, Handler, WireError};
use vessel_session::terminals::{AttentionError, TerminalHandle};

#[derive(Debug)]
pub(crate) struct ActivityProducer(TerminalHandle);
impl ActivityProducer {
    pub(crate) const fn new(handle: TerminalHandle) -> Self {
        Self(handle)
    }
    fn report(&self, request: ReportTerminalActivity) -> Result<TerminalAttention, WireError> {
        if request.terminal != self.0.id() {
            return Err(WireError::new(
                ErrorCode::Forbidden,
                "this activity credential belongs to another terminal",
            ));
        }
        let revision = request
            .revision
            .or_else(|| self.0.info().attention.map(|attention| attention.revision))
            .ok_or_else(|| {
                WireError::new(ErrorCode::Conflict, "terminal activity is not available")
            })?;
        self.0
            .report_activity(revision, request.signal)
            .map_err(|error| {
                let code = match error {
                    AttentionError::Stale | AttentionError::Ended => ErrorCode::Conflict,
                    AttentionError::Exhausted => ErrorCode::Unavailable,
                };
                WireError::new(code, error.to_string())
            })
    }
}
macro_rules! activity_method {
    (ReportTerminalActivity,$handler:ident,$request:ty,$response:ty) => {
        async fn $handler(&self, _context: &(), request: $request) -> Result<$response, WireError> {
            self.report(request)
        }
    };
    ($variant:ident,$handler:ident,$request:ty,$response:ty) => {
        async fn $handler(
            &self,
            _context: &(),
            _request: $request,
        ) -> Result<$response, WireError> {
            Err(WireError::new(
                ErrorCode::Forbidden,
                "terminal activity credentials cannot call this node method",
            ))
        }
    };
}
macro_rules! activity_handler {
    ($( $(#[$doc:meta])* $variant:ident / $handler:ident / $wire:literal [$scope:ident,$protocol:tt,$shape:ident]:$request:ty => $response:ty; )*) => {
        impl Handler for ActivityProducer {
            type Context = ();
            $( activity_method!($variant,$handler,$request,$response); )*
        }
    };
}
vessel_proto::method_table!(activity_handler);

pub(crate) fn producer_auth(
    terminal: vessel_proto::TerminalId,
) -> Result<Option<vessel_proto::notifications::TerminalActivityAuth>, super::TerminalsError> {
    if vessel_proto::notifications::ATTENTION_NOTIFICATIONS_PROTOCOL.is_none() {
        return Ok(None);
    }
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(super::TerminalsError::Random)?;
    Ok(Some(vessel_proto::notifications::TerminalActivityAuth {
        terminal,
        secret: vessel_proto::notifications::TerminalActivitySecret::from_bytes(bytes),
    }))
}
pub(crate) fn producer_env(
    spec: vessel_proc::CommandSpec,
    auth: Option<&vessel_proto::notifications::TerminalActivityAuth>,
    endpoint: &std::path::Path,
) -> vessel_proc::CommandSpec {
    let Some(auth) = auth else {
        return spec;
    };
    spec.env(
        vessel_proto::notifications::TERMINAL_ACTIVITY_ID_ENV,
        auth.terminal.to_string(),
    )
    .env(
        vessel_proto::notifications::TERMINAL_ACTIVITY_SECRET_ENV,
        auth.secret.to_string(),
    )
    .env(
        vessel_proto::notifications::TERMINAL_ACTIVITY_SOCKET_ENV,
        endpoint.as_os_str(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_util::{sync::CancellationToken, task::TaskTracker};
    use vessel_proto::notifications::{
        ReportTerminalActivity, TerminalActivity, TerminalActivitySignal, TerminalAttentionReason,
        TerminalAttentionRevision,
    };
    use vessel_session::terminals::{ScriptStart, Scrollback, start_script};
    fn terminal(tasks: &TaskTracker, stop: &CancellationToken) -> TerminalHandle {
        let id = vessel_proto::TerminalId::random();
        start_script(ScriptStart {
            info: vessel_proto::terminals::TerminalInfo::new(
                id,
                vessel_proto::terminals::TerminalPlace::Session {
                    session: vessel_proto::SessionId::random(),
                },
                std::path::PathBuf::from("/synthetic"),
                vessel_proto::terminals::TerminalKind::Shell {
                    program: std::path::PathBuf::from("/synthetic-not-executable"),
                },
                chrono::DateTime::UNIX_EPOCH,
            ),
            spec: vessel_proc::CommandSpec::new("/synthetic-not-executable"),
            leftover: vessel_session::terminals::Leftover::default(),
            service: false,
            approved: || true,
            scrollback: Scrollback::DEFAULT,
            shutdown: stop.clone(),
            tasks: tasks.clone(),
        })
    }
    #[tokio::test(flavor = "current_thread")]
    async fn activity_capability_cannot_read_sessions_claim_notifications_or_report_another_terminal()
     {
        // The current-thread actor remains unpolled until the final wait;
        // its nonexistent command can never start a process on any platform.
        let tasks = TaskTracker::new();
        let stop = CancellationToken::new();
        let handle = terminal(&tasks, &stop);
        let id = handle.id();
        let producer = ActivityProducer::new(handle.clone());
        assert_eq!(
            producer
                .device_self(&(), vessel_proto::GetDeviceSelf {})
                .await
                .unwrap_err()
                .code,
            ErrorCode::Forbidden
        );
        assert_eq!(
            producer
                .get_session(
                    &(),
                    vessel_proto::GetSession {
                        session: vessel_proto::SessionId::random()
                    }
                )
                .await
                .unwrap_err()
                .code,
            ErrorCode::Forbidden
        );
        assert_eq!(
            producer
                .claim_notification(
                    &(),
                    vessel_proto::notifications::ClaimNotification {
                        source: vessel_proto::notifications::NotificationSource::Terminal {
                            terminal: id,
                            revision: TerminalAttentionRevision::new(0)
                        }
                    }
                )
                .await
                .unwrap_err()
                .code,
            ErrorCode::Forbidden
        );
        assert_eq!(
            producer
                .report(ReportTerminalActivity {
                    terminal: vessel_proto::TerminalId::random(),
                    revision: None,
                    signal: TerminalActivitySignal::Working
                })
                .unwrap_err()
                .code,
            ErrorCode::Forbidden
        );
        let working = producer
            .report(ReportTerminalActivity {
                terminal: id,
                revision: None,
                signal: TerminalActivitySignal::Working,
            })
            .unwrap();
        assert_eq!(
            producer
                .report(ReportTerminalActivity {
                    terminal: id,
                    revision: Some(TerminalAttentionRevision::new(0)),
                    signal: TerminalActivitySignal::Idle
                })
                .unwrap_err()
                .code,
            ErrorCode::Conflict
        );
        let finished = producer
            .report(ReportTerminalActivity {
                terminal: id,
                revision: Some(working.revision),
                signal: TerminalActivitySignal::Idle,
            })
            .unwrap();
        assert_eq!(
            finished.activity,
            TerminalActivity::Attention {
                reason: TerminalAttentionReason::Finished
            }
        );
        stop.cancel();
        let _closed = tasks.close();
        tasks.wait().await;
    }
    #[test]
    fn environment_injection_does_not_expose_the_capability_through_debug() {
        let secret = vessel_proto::notifications::TerminalActivitySecret::from_bytes([7; 32]);
        let auth = vessel_proto::notifications::TerminalActivityAuth {
            terminal: vessel_proto::TerminalId::random(),
            secret: secret.clone(),
        };
        let spec = producer_env(
            vessel_proc::CommandSpec::new("/synthetic"),
            Some(&auth),
            std::path::Path::new("/synthetic/node.sock"),
        );
        assert!(
            spec.env_change(vessel_proto::notifications::TERMINAL_ACTIVITY_SECRET_ENV)
                .is_some()
        );
        assert!(!format!("{spec:?}").contains(&secret.to_string()));
    }
}
