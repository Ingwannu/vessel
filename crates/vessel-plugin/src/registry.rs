//! The handlers a node has registered, in plugin-id order.
//!
//! Generic over the handler so a release build can hold [`Idle`] and a
//! test can hold its own type. Nothing here is a trait object.

use std::future::Future;

use tokio::pin;
use tokio_util::sync::CancellationToken;
use vessel_proto::SessionSpec;

use crate::check::locked_change;
use crate::error::{HandlerStop, PluginError};
use crate::hook::Hook;
use crate::id::PluginId;

/// What `agent.create.before` calls.
pub trait BeforeHandler: Send + Sync {
    /// Run one handler. `Ok(None)` keeps the spec. `Ok(Some)` proposes a
    /// replacement. `Err` refuses, and the error must not carry the
    /// handler's text: [`HandlerStop`] has none.
    fn run(
        &self,
        spec: SessionSpec,
        cancel: &CancellationToken,
    ) -> impl Future<Output = Result<Option<SessionSpec>, HandlerStop>> + Send;
}

/// What `agent.create.after` calls.
pub trait AfterHandler: Send + Sync {
    /// Run one notification. A refusal is logged by the registry and the
    /// next handler still runs. The spec is borrowed so a log of this
    /// call cannot need to store it.
    fn run(
        &self,
        spec: &SessionSpec,
        cancel: &CancellationToken,
    ) -> impl Future<Output = Result<(), HandlerStop>> + Send;
}

/// A handler that is never registered. Its body keeps the spec so a
/// mistaken registration would still not rewrite a create.
#[derive(Debug, Clone, Copy, Default)]
pub struct Idle;

impl BeforeHandler for Idle {
    #[expect(
        clippy::unused_async,
        reason = "nothing is registered, so the handler returns without waiting"
    )]
    async fn run(
        &self,
        _spec: SessionSpec,
        _cancel: &CancellationToken,
    ) -> Result<Option<SessionSpec>, HandlerStop> {
        Ok(None)
    }
}

impl AfterHandler for Idle {
    #[expect(
        clippy::unused_async,
        reason = "nothing is registered, so the handler returns without waiting"
    )]
    async fn run(
        &self,
        _spec: &SessionSpec,
        _cancel: &CancellationToken,
    ) -> Result<(), HandlerStop> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct Entry<H> {
    id: PluginId,
    registered: u32,
    handler: H,
}

/// Handlers for the two create hooks this slice calls.
#[derive(Debug, Clone)]
pub struct Registry<B, A> {
    before: Vec<Entry<B>>,
    after: Vec<Entry<A>>,
    next: u32,
}

impl<B, A> Default for Registry<B, A> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<B, A> Registry<B, A> {
    /// No handlers. Create is unchanged.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            before: Vec::new(),
            after: Vec::new(),
            next: 0,
        }
    }

    /// Whether `agent.create.before` would call anything.
    #[must_use]
    pub const fn before_is_empty(&self) -> bool {
        self.before.is_empty()
    }

    /// Whether `agent.create.after` would call anything.
    #[must_use]
    pub const fn after_is_empty(&self) -> bool {
        self.after.is_empty()
    }

    /// Record a before handler. Order among one id is registration order.
    ///
    /// # Errors
    ///
    /// The registration counter no longer fits in a `u32`.
    pub fn register_before(&mut self, id: PluginId, handler: B) -> Result<(), PluginError> {
        let registered = self.ticket()?;
        self.before.push(Entry {
            id,
            registered,
            handler,
        });
        Ok(())
    }

    /// Record an after handler.
    ///
    /// # Errors
    ///
    /// The registration counter no longer fits in a `u32`.
    pub fn register_after(&mut self, id: PluginId, handler: A) -> Result<(), PluginError> {
        let registered = self.ticket()?;
        self.after.push(Entry {
            id,
            registered,
            handler,
        });
        Ok(())
    }

    fn ticket(&mut self) -> Result<u32, PluginError> {
        let issued = self.next;
        self.next = self.next.checked_add(1).ok_or(PluginError::Full)?;
        Ok(issued)
    }
}

impl<B, A> Registry<B, A>
where
    B: BeforeHandler,
{
    /// Run `agent.create.before`. An empty registry returns `spec`.
    ///
    /// # Errors
    ///
    /// A handler refused, replaced a locked field, or the token was
    /// cancelled. Later handlers do not run.
    pub async fn before(
        &self,
        mut spec: SessionSpec,
        cancel: &CancellationToken,
    ) -> Result<SessionSpec, PluginError> {
        for index in ranked(&self.before) {
            if cancel.is_cancelled() {
                return Err(self.cancelled(index, Hook::AgentCreateBefore)?);
            }
            spec = self.one_before(index, spec, cancel).await?;
        }
        Ok(spec)
    }

    async fn one_before(
        &self,
        index: u32,
        spec: SessionSpec,
        cancel: &CancellationToken,
    ) -> Result<SessionSpec, PluginError> {
        let id = self.before_id(index)?;
        let Some(entry) = entry_at(&self.before, index) else {
            return Err(PluginError::Broken);
        };
        let call = entry.handler.run(spec.clone(), cancel);
        pin!(call);
        let outcome = tokio::select! {
            biased;
            () = cancel.cancelled() => {
                return Err(PluginError::Cancelled { plugin: id, hook: Hook::AgentCreateBefore });
            }
            result = &mut call => result,
        };
        match outcome {
            Ok(None) => Ok(spec),
            Ok(Some(next)) => applied(id, &spec, next),
            Err(HandlerStop) => Err(PluginError::Refused {
                plugin: id,
                hook: Hook::AgentCreateBefore,
            }),
        }
    }

    fn cancelled(&self, index: u32, hook: Hook) -> Result<PluginError, PluginError> {
        Ok(PluginError::Cancelled {
            plugin: self.before_id(index)?,
            hook,
        })
    }

    fn before_id(&self, index: u32) -> Result<PluginId, PluginError> {
        entry_at(&self.before, index)
            .map(|entry| entry.id.clone())
            .ok_or(PluginError::Broken)
    }
}

impl<B, A> Registry<B, A>
where
    A: AfterHandler,
{
    /// Run `agent.create.after`. Failures are logged and the loop goes on.
    /// Cancellation stops the loop. The spec is not logged.
    pub async fn after(&self, spec: SessionSpec, cancel: &CancellationToken) {
        for index in ranked(&self.after) {
            if cancel.is_cancelled() {
                return;
            }
            self.one_after(index, &spec, cancel).await;
        }
    }

    async fn one_after(&self, index: u32, spec: &SessionSpec, cancel: &CancellationToken) {
        let Some(entry) = entry_at(&self.after, index) else {
            tracing::warn!("a create hook entry was missing");
            return;
        };
        let plugin = entry.id.clone();
        let call = entry.handler.run(spec, cancel);
        pin!(call);
        let outcome = tokio::select! {
            biased;
            () = cancel.cancelled() => return,
            result = &mut call => result,
        };
        if outcome.is_err() {
            tracing::warn!(%plugin, hook = %Hook::AgentCreateAfter, "a create hook failed");
        }
    }
}

fn applied(
    plugin: PluginId,
    previous: &SessionSpec,
    next: SessionSpec,
) -> Result<SessionSpec, PluginError> {
    if let Some(field) = locked_change(previous, &next) {
        return Err(PluginError::Changed {
            plugin,
            hook: Hook::AgentCreateBefore,
            field,
        });
    }
    Ok(next)
}

fn ranked<H>(entries: &[Entry<H>]) -> Vec<u32> {
    let mut order = Vec::new();
    for (index, _) in entries.iter().enumerate() {
        if let Ok(index) = u32::try_from(index) {
            order.push(index);
        }
    }
    order.sort_by(|left, right| rank_key(entries, *left).cmp(&rank_key(entries, *right)));
    order
}

fn rank_key<H>(entries: &[Entry<H>], index: u32) -> Option<(&PluginId, u32)> {
    let entry = entry_at(entries, index)?;
    Some((&entry.id, entry.registered))
}

fn entry_at<H>(entries: &[Entry<H>], index: u32) -> Option<&Entry<H>> {
    let index = usize::try_from(index).ok()?;
    entries.get(index)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use tokio_util::sync::CancellationToken;
    use vessel_proto::{HarnessKind, SessionMode, SessionSpec, SessionTitle};

    use super::{AfterHandler, BeforeHandler, Registry};
    use crate::error::{HandlerStop, PluginError};
    use crate::id::PluginId;

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
    struct Mark {
        label: u8,
        log: Arc<Mutex<Vec<u8>>>,
        change_cwd: bool,
        refuse: bool,
    }

    impl BeforeHandler for Mark {
        #[expect(
            clippy::unused_async,
            reason = "the test records the call and returns; it does not wait"
        )]
        async fn run(
            &self,
            mut spec: SessionSpec,
            _cancel: &CancellationToken,
        ) -> Result<Option<SessionSpec>, HandlerStop> {
            self.log.lock().expect("log").push(self.label);
            if self.refuse {
                return Err(HandlerStop);
            }
            if self.change_cwd {
                spec.cwd = Some(PathBuf::from("/elsewhere"));
            }
            Ok(Some(spec))
        }
    }

    fn mark(label: u8, log: &Arc<Mutex<Vec<u8>>>) -> Mark {
        Mark {
            label,
            log: Arc::clone(log),
            change_cwd: false,
            refuse: false,
        }
    }

    #[tokio::test]
    async fn an_empty_registry_keeps_the_spec() {
        let hooks: Registry<Mark, Mark> = Registry::empty();
        let cancel = CancellationToken::new();
        let original = spec();
        let got = hooks.before(original.clone(), &cancel).await.unwrap();
        assert_eq!(got, original);
    }

    #[tokio::test]
    async fn ids_run_in_byte_order_then_registration_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut hooks = Registry::<Mark, Mark>::empty();
        hooks
            .register_before(PluginId::parse("m").unwrap(), mark(1, &log))
            .unwrap();
        hooks
            .register_before(PluginId::parse("a").unwrap(), mark(2, &log))
            .unwrap();
        hooks
            .register_before(PluginId::parse("a").unwrap(), mark(3, &log))
            .unwrap();
        let cancel = CancellationToken::new();
        hooks.before(spec(), &cancel).await.unwrap();
        assert_eq!(log.lock().expect("log").as_slice(), [2, 3, 1]);
    }

    #[tokio::test]
    async fn a_title_change_is_kept() {
        let mut hooks = Registry::<Title, Mark>::empty();
        hooks
            .register_before(PluginId::parse("a").unwrap(), Title)
            .unwrap();
        let cancel = CancellationToken::new();
        let got = hooks.before(spec(), &cancel).await.unwrap();
        assert_eq!(got.title, SessionTitle::from_prompt("renamed"));
        assert_eq!(got.cwd, spec().cwd);
    }

    struct Title;

    impl BeforeHandler for Title {
        #[expect(clippy::unused_async, reason = "the test only replaces the title")]
        async fn run(
            &self,
            mut spec: SessionSpec,
            _cancel: &CancellationToken,
        ) -> Result<Option<SessionSpec>, HandlerStop> {
            spec.title = SessionTitle::from_prompt("renamed");
            Ok(Some(spec))
        }
    }

    #[tokio::test]
    async fn a_locked_field_stops_later_handlers_and_hides_the_path() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut hooks = Registry::<Mark, Mark>::empty();
        let mut first = mark(1, &log);
        first.change_cwd = true;
        hooks
            .register_before(PluginId::parse("a").unwrap(), first)
            .unwrap();
        hooks
            .register_before(PluginId::parse("b").unwrap(), mark(2, &log))
            .unwrap();
        let error = hooks
            .before(spec(), &CancellationToken::new())
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            PluginError::Changed {
                field: crate::error::LockedField::Cwd,
                ..
            }
        ));
        let message = error.to_string();
        assert!(message.contains("cwd"));
        assert!(!message.contains("secret"));
        assert_eq!(log.lock().expect("log").as_slice(), [1]);
    }

    #[tokio::test]
    async fn a_refusal_stops_later_handlers() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut hooks = Registry::<Mark, Mark>::empty();
        let mut first = mark(1, &log);
        first.refuse = true;
        hooks
            .register_before(PluginId::parse("a").unwrap(), first)
            .unwrap();
        hooks
            .register_before(PluginId::parse("b").unwrap(), mark(2, &log))
            .unwrap();
        let error = hooks
            .before(spec(), &CancellationToken::new())
            .await
            .unwrap_err();
        assert!(matches!(error, PluginError::Refused { .. }));
        assert_eq!(log.lock().expect("log").as_slice(), [1]);
    }

    #[derive(Clone)]
    struct AfterMark {
        label: u8,
        log: Arc<Mutex<Vec<u8>>>,
        refuse: bool,
    }

    impl AfterHandler for AfterMark {
        #[expect(
            clippy::unused_async,
            reason = "the test records the call and returns; it does not wait"
        )]
        async fn run(
            &self,
            _spec: &SessionSpec,
            _cancel: &CancellationToken,
        ) -> Result<(), HandlerStop> {
            self.log.lock().expect("log").push(self.label);
            if self.refuse {
                Err(HandlerStop)
            } else {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn an_after_failure_does_not_stop_the_next_handler() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut hooks = Registry::<Mark, AfterMark>::empty();
        hooks
            .register_after(
                PluginId::parse("a").unwrap(),
                AfterMark {
                    label: 1,
                    log: Arc::clone(&log),
                    refuse: true,
                },
            )
            .unwrap();
        hooks
            .register_after(
                PluginId::parse("b").unwrap(),
                AfterMark {
                    label: 2,
                    log: Arc::clone(&log),
                    refuse: false,
                },
            )
            .unwrap();
        hooks.after(spec(), &CancellationToken::new()).await;
        assert_eq!(log.lock().expect("log").as_slice(), [1, 2]);
    }

    #[tokio::test]
    async fn a_cancelled_before_hook_names_the_plugin() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut hooks = Registry::<Mark, Mark>::empty();
        hooks
            .register_before(PluginId::parse("slow").unwrap(), mark(1, &log))
            .unwrap();
        let cancel = CancellationToken::new();
        cancel.cancel();
        let error = hooks.before(spec(), &cancel).await.unwrap_err();
        assert!(matches!(
            error,
            PluginError::Cancelled { ref plugin, hook: crate::hook::Hook::AgentCreateBefore }
                if plugin.as_str() == "slow"
        ));
    }
}
