//! The two create hooks on the shared session create (ADR 0068).
//!
//! `agent.create.before` runs after the draining refusal and before the
//! checks that read the spec. The node counts the 30 seconds. A release
//! registry is [`Idle`](vessel_plugin::Idle) and empty, so create is
//! unchanged. Test handlers live only in this module's tests.

use tokio_util::sync::CancellationToken;
use vessel_plugin::{Idle, PluginError, Registry, WaitingHook};
use vessel_proto::{ErrorCode, SessionOrigin, SessionSpec, WireError};

/// The registry a release build holds: no handler can be constructed but
/// [`Idle`](Idle), and nothing registers one.
pub(super) type Hooks = Registry<Idle, Idle>;

/// Why a cancelled hook ended. A boolean would not say which message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CancelCause {
    /// The 30-second deadline passed.
    Deadline,
    /// The node is stopping.
    Stopping,
}

/// Run `agent.create.before` unless this create is an import (`origin` is
/// set). An empty registry returns `spec` without waiting.
///
/// # Errors
///
/// A refusal is `InvalidRequest`. A timeout or a stop is `Unavailable`.
/// The message names the plugin and the hook, and the field when a locked
/// field changed. It does not include the handler's text or a path.
pub(super) async fn rewrite_create<B, A>(
    hooks: &Registry<B, A>,
    stop: &CancellationToken,
    origin: Option<&SessionOrigin>,
    spec: SessionSpec,
) -> Result<SessionSpec, WireError>
where
    B: vessel_plugin::BeforeHandler,
    A: vessel_plugin::AfterHandler,
{
    if origin.is_some() || hooks.before_is_empty() {
        return Ok(spec);
    }
    let cancel = stop.child_token();
    let child = cancel.clone();
    let run = hooks.before(spec, &cancel);
    tokio::pin!(run);
    match tokio::time::timeout(WaitingHook::LIMIT.duration(), &mut run).await {
        Ok(Ok(spec)) => Ok(spec),
        Ok(Err(error)) => Err(wire_hook(&error, cause_of(stop))),
        Err(_elapsed) => {
            child.cancel();
            match run.await {
                Ok(spec) => Ok(spec),
                Err(error) => Err(wire_hook(&error, CancelCause::Deadline)),
            }
        }
    }
}

/// Start `agent.create.after` and return. The create that already
/// succeeded does not wait. An import does not fire it. The spawned task
/// ends when `stop` is cancelled.
pub(super) fn note_created<B, A>(
    hooks: &Registry<B, A>,
    stop: &CancellationToken,
    origin: Option<&SessionOrigin>,
    spec: SessionSpec,
) where
    B: vessel_plugin::BeforeHandler + Clone + Send + Sync + 'static,
    A: vessel_plugin::AfterHandler + Clone + Send + Sync + 'static,
{
    if origin.is_some() || hooks.after_is_empty() {
        return;
    }
    let hooks = hooks.clone();
    let cancel = stop.child_token();
    let work = async move {
        hooks.after(spec, &cancel).await;
    };
    // Dropping the handle detaches the task: the create that already
    // succeeded does not wait, and the task is not aborted.
    drop(tokio::spawn(work));
}

fn cause_of(stop: &CancellationToken) -> CancelCause {
    if stop.is_cancelled() {
        CancelCause::Stopping
    } else {
        CancelCause::Deadline
    }
}

fn wire_hook(error: &PluginError, cause: CancelCause) -> WireError {
    match error {
        PluginError::Refused { plugin, hook } => WireError::new(
            ErrorCode::InvalidRequest,
            format!("plugin {plugin} hook {hook} refused"),
        ),
        PluginError::Changed {
            plugin,
            hook,
            field,
        } => WireError::new(
            ErrorCode::InvalidRequest,
            format!("plugin {plugin} hook {hook} refused to change {field}"),
        ),
        PluginError::Cancelled { plugin, hook } => {
            let why = match cause {
                CancelCause::Deadline => "timed out",
                CancelCause::Stopping => "stopped because the node is stopping",
            };
            WireError::new(
                ErrorCode::Unavailable,
                format!("plugin {plugin} hook {hook} {why}"),
            )
        }
        PluginError::EmptyId | PluginError::BadId | PluginError::Full | PluginError::Broken => {
            WireError::new(ErrorCode::InvalidRequest, error.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;

    use tokio::sync::Notify;
    use tokio_util::sync::CancellationToken;
    use vessel_plugin::{AfterHandler, BeforeHandler, HandlerStop, PluginId, Registry};
    use vessel_proto::{
        ErrorCode, HarnessKind, NodeId, SessionId, SessionMode, SessionOrigin, SessionSpec,
    };

    use super::{note_created, rewrite_create};

    fn spec() -> SessionSpec {
        SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Structured {
                model: None,
                resume: None,
                options: Vec::new(),
            },
        )
        .with_cwd(PathBuf::from("/plugin-secret-dir"))
    }

    #[derive(Clone)]
    struct Hang;

    impl BeforeHandler for Hang {
        async fn run(
            &self,
            spec: SessionSpec,
            cancel: &CancellationToken,
        ) -> Result<Option<SessionSpec>, HandlerStop> {
            cancel.cancelled().await;
            Ok(Some(spec))
        }
    }

    impl AfterHandler for Hang {
        async fn run(
            &self,
            _spec: &SessionSpec,
            cancel: &CancellationToken,
        ) -> Result<(), HandlerStop> {
            cancel.cancelled().await;
            Ok(())
        }
    }

    #[derive(Clone)]
    struct Refuse;

    impl BeforeHandler for Refuse {
        #[expect(clippy::unused_async, reason = "the test refuses without waiting")]
        async fn run(
            &self,
            _spec: SessionSpec,
            _cancel: &CancellationToken,
        ) -> Result<Option<SessionSpec>, HandlerStop> {
            Err(HandlerStop)
        }
    }

    impl AfterHandler for Refuse {
        #[expect(clippy::unused_async, reason = "the test refuses without waiting")]
        async fn run(
            &self,
            _spec: &SessionSpec,
            _cancel: &CancellationToken,
        ) -> Result<(), HandlerStop> {
            Err(HandlerStop)
        }
    }

    #[derive(Clone)]
    struct MoveDir;

    impl BeforeHandler for MoveDir {
        #[expect(clippy::unused_async, reason = "the test only replaces the directory")]
        async fn run(
            &self,
            mut spec: SessionSpec,
            _cancel: &CancellationToken,
        ) -> Result<Option<SessionSpec>, HandlerStop> {
            spec.cwd = Some(PathBuf::from("/elsewhere"));
            Ok(Some(spec))
        }
    }

    impl AfterHandler for MoveDir {
        #[expect(
            clippy::unused_async,
            reason = "the after hook is not what this test calls"
        )]
        async fn run(
            &self,
            _spec: &SessionSpec,
            _cancel: &CancellationToken,
        ) -> Result<(), HandlerStop> {
            Ok(())
        }
    }

    #[derive(Clone)]
    struct WaitAfter {
        started: Arc<Notify>,
        release: Arc<Notify>,
        finished: Arc<Notify>,
    }

    impl BeforeHandler for WaitAfter {
        #[expect(
            clippy::unused_async,
            reason = "this test only waits in the after hook"
        )]
        async fn run(
            &self,
            spec: SessionSpec,
            _cancel: &CancellationToken,
        ) -> Result<Option<SessionSpec>, HandlerStop> {
            Ok(Some(spec))
        }
    }

    impl AfterHandler for WaitAfter {
        async fn run(
            &self,
            _spec: &SessionSpec,
            _cancel: &CancellationToken,
        ) -> Result<(), HandlerStop> {
            self.started.notify_one();
            self.release.notified().await;
            self.finished.notify_one();
            Ok(())
        }
    }

    #[tokio::test(start_paused = true)]
    async fn a_waiting_hook_times_out_at_thirty_seconds() {
        let mut hooks = Registry::<Hang, Hang>::empty();
        hooks
            .register_before(PluginId::parse("slow").unwrap(), Hang)
            .unwrap();
        let stop = CancellationToken::new();
        let task = tokio::spawn(async move { rewrite_create(&hooks, &stop, None, spec()).await });
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(29)).await;
        assert!(!task.is_finished(), "29s is inside the deadline");
        tokio::time::advance(Duration::from_secs(1)).await;
        let error = task.await.unwrap().unwrap_err();
        assert_eq!(error.code, ErrorCode::Unavailable);
        assert!(error.message.contains("slow"));
        assert!(error.message.contains("agent.create.before"));
        assert!(error.message.contains("timed out"));
        assert!(!error.message.contains("secret"));
    }

    #[tokio::test]
    async fn a_refusal_is_invalid_request() {
        let mut hooks = Registry::<Refuse, Refuse>::empty();
        hooks
            .register_before(PluginId::parse("gate").unwrap(), Refuse)
            .unwrap();
        let error = rewrite_create(&hooks, &CancellationToken::new(), None, spec())
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidRequest);
        assert!(error.message.contains("gate"));
        assert!(error.message.contains("agent.create.before"));
        assert!(error.message.contains("refused"));
        assert!(!error.message.contains("secret"));
    }

    #[tokio::test]
    async fn a_directory_change_names_the_field() {
        let mut hooks = Registry::<MoveDir, MoveDir>::empty();
        hooks
            .register_before(PluginId::parse("gate").unwrap(), MoveDir)
            .unwrap();
        let error = rewrite_create(&hooks, &CancellationToken::new(), None, spec())
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidRequest);
        assert!(error.message.contains("refused to change cwd"));
        assert!(!error.message.contains("secret"));
        assert!(!error.message.contains("elsewhere"));
    }

    #[tokio::test]
    async fn an_import_does_not_run_the_hook() {
        let mut hooks = Registry::<Refuse, Refuse>::empty();
        hooks
            .register_before(PluginId::parse("gate").unwrap(), Refuse)
            .unwrap();
        let origin = SessionOrigin::Imported {
            node: NodeId::random(),
            session: SessionId::random(),
        };
        let original = spec();
        let got = rewrite_create(
            &hooks,
            &CancellationToken::new(),
            Some(&origin),
            original.clone(),
        )
        .await
        .unwrap();
        assert_eq!(got, original);
    }

    #[tokio::test]
    async fn create_after_does_not_block_the_caller() {
        let started = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let finished = Arc::new(Notify::new());
        let mut hooks = Registry::<WaitAfter, WaitAfter>::empty();
        hooks
            .register_after(
                PluginId::parse("later").unwrap(),
                WaitAfter {
                    started: Arc::clone(&started),
                    release: Arc::clone(&release),
                    finished: Arc::clone(&finished),
                },
            )
            .unwrap();
        note_created(&hooks, &CancellationToken::new(), None, spec());
        started.notified().await;
        release.notify_one();
        finished.notified().await;
    }
}
