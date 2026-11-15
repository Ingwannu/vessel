//! Opaque bounded App fixture: the Link owns its one runtime; the driver owns
//! its one real State and framed peer. Test code receives no authority factory.
use super::{FixtureError, Home, Phase, failure, make_state, peer};
use crate::{Command, Event, Link};
use tokio::{
    sync::{mpsc, oneshot, watch},
    task::JoinHandle,
};
use vessel_app_view::{connection::ConnectionContext, machine_book::Name};
use vessel_proto::{Call, Outcome};
mod driver;

const CONTROL_CAPACITY: usize = 4;
const RUNTIME_WORKERS: usize = 2;
const OUTER: std::time::Duration = std::time::Duration::from_secs(30);

/// Whether a genuine replacement peer reports the old or a different node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reconnect {
    /// The same node with a fresh admitted epoch.
    SameNode,
    /// A different node under the same explicitly selected alias.
    DifferentNode,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Lifetime {
    Active,
    Stopped,
}
type Port = oneshot::Sender<Result<Response, FixtureError>>;
enum Control {
    Request(Port),
    Reply(Box<Outcome>, Port),
    Event(Port),
    Retire(Port),
    Reconnect(Reconnect, Port),
    ClosePeer(Port),
    Transcript(Port),
    Stop(Port),
}
enum Response {
    Context(ConnectionContext),
    Call(Box<Call>),
    Event(Box<Event>),
    Transcript(Vec<Call>),
    Done,
}

/// An explicit regression scenario over real framing and canonical admission.
/// Every blocking test operation borrows the original Link, keeping its single
/// runtime alive. Never use this facade from production UI event handlers.
pub struct Scenario {
    context: ConnectionContext,
    controls: mpsc::Sender<Control>,
    original: mpsc::WeakSender<Command>,
    stop: watch::Sender<Lifetime>,
    task: Option<JoinHandle<Result<(), FixtureError>>>,
}
impl Scenario {
    /// Start one isolated actor on a single real Link queue/runtime.
    ///
    /// # Errors
    /// Refuses nested runtimes, local setup failure and bounded handshake failure.
    pub fn start(name: &Name) -> Result<(Link, Self), FixtureError> {
        outside_runtime()?;
        // A detached queue intentionally has no I/O/time driver. This is
        // one real enabled Link runtime, with the actual State attached below.
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(RUNTIME_WORKERS)
            .thread_name("vessel-scenario")
            .enable_all()
            .build()
            .map_err(|source| FixtureError::Io {
                phase: Phase::Setup,
                source,
            })?;
        let (commands_out, commands) = mpsc::channel(crate::CHANNEL);
        let link = Link {
            commands: commands_out,
            _runtime: runtime,
        };
        let home = Home::try_new()?;
        let (events, outbox) = mpsc::channel(crate::CHANNEL);
        let mut state = make_state(&home, link.commands.downgrade(), events);
        let (peer, context) = owned_runtime(&link).block_on(async {
            let mut peer =
                peer::try_controlled(name.as_str(), vessel_proto::NodeId::random()).await?;
            let context = driver::admit(&mut state, &mut peer, name.as_str())?;
            Ok::<_, FixtureError>((peer, context))
        })?;
        let (controls, control_inbox) = mpsc::channel(CONTROL_CAPACITY);
        let (stop, stopped) = watch::channel(Lifetime::Active);
        let driver = driver::Driver::new(
            driver::Seed {
                state,
                peer,
                home,
                context: context.clone(),
            },
            driver::Channels {
                commands,
                events: outbox,
                controls: control_inbox,
                stopped,
            },
        );
        let task = owned_runtime(&link).spawn(async move {
            failure::phase_with_limit(Phase::Control, OUTER, driver.run()).await?
        });
        let scenario = Self {
            context,
            controls,
            original: link.commands.downgrade(),
            stop,
            task: Some(task),
        };
        Ok((link, scenario))
    }
    /// Original public facts only; permission and validity are never inferred.
    #[must_use]
    pub fn context(&self) -> ConnectionContext {
        self.context.clone()
    }

    fn action(
        &mut self,
        link: &Link,
        make: impl FnOnce(Port) -> Control,
    ) -> Result<Response, FixtureError> {
        outside_runtime()?;
        let original = self
            .original
            .upgrade()
            .ok_or(FixtureError::Closed(Phase::Control))?;
        if !original.same_channel(&link.commands) {
            return Err(FixtureError::WrongLink);
        }
        drop(original);
        let (send, receive) = oneshot::channel();
        self.controls
            .try_send(make(send))
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_control) => FixtureError::Full,
                mpsc::error::TrySendError::Closed(_control) => FixtureError::Closed(Phase::Control),
            })?;
        owned_runtime(link).block_on(async {
            failure::phase(Phase::Control, receive)
                .await?
                .map_err(|_closed| FixtureError::Closed(Phase::Control))?
        })
    }
    /// Wait for a real framed request; no synthetic command shortcut.
    ///
    /// # Errors
    /// The bounded peer/actor operation failed or a prior call is pending.
    pub fn request(&mut self, link: &Link) -> Result<Call, FixtureError> {
        match self.action(link, Control::Request)? {
            Response::Call(call) => Ok(*call),
            Response::Context(_)
            | Response::Event(_)
            | Response::Transcript(_)
            | Response::Done => Err(FixtureError::Protocol),
        }
    }
    /// Answer the actual pending frame through its original correlation.
    ///
    /// # Errors
    /// No call is pending or its transport has ended.
    pub fn reply(&mut self, link: &Link, outcome: Outcome) -> Result<(), FixtureError> {
        self.done(link, |port| Control::Reply(Box::new(outcome), port))
    }
    /// Receive an event produced by the real State/Devices worker.
    ///
    /// # Errors
    /// The event phase failed or another receive is already pending.
    pub fn event(&mut self, link: &Link) -> Result<Event, FixtureError> {
        match self.action(link, Control::Event)? {
            Response::Event(event) => Ok(*event),
            Response::Context(_) | Response::Call(_) | Response::Transcript(_) | Response::Done => {
                Err(FixtureError::Protocol)
            }
        }
    }
    /// Retire the exact original context through the existing actor command.
    ///
    /// # Errors
    /// Dispatch or its bounded publication phase failed.
    pub fn retire(&mut self, link: &Link) -> Result<(), FixtureError> {
        self.done(link, Control::Retire)
    }
    /// Replace the selected original admission, never search aliases by node.
    ///
    /// # Errors
    /// The old peer could not close or the new framed handshake/admission failed.
    pub fn reconnect(
        &mut self,
        link: &Link,
        change: Reconnect,
    ) -> Result<ConnectionContext, FixtureError> {
        match self.action(link, |port| Control::Reconnect(change, port))? {
            Response::Context(context) => {
                self.context = context.clone();
                Ok(context)
            }
            Response::Call(_) | Response::Event(_) | Response::Transcript(_) | Response::Done => {
                Err(FixtureError::Protocol)
            }
        }
    }
    /// Close the actual peer transport, keeping actor retirement independent.
    ///
    /// # Errors
    /// Its bounded cancellation/join phase failed.
    pub fn close_peer(&mut self, link: &Link) -> Result<(), FixtureError> {
        self.done(link, Control::ClosePeer)
    }
    /// Bounded complete calls, for exact no-replay/no-extra-frame assertions.
    ///
    /// # Errors
    /// The scenario ended or its transcript lock was poisoned.
    pub fn transcript(&mut self, link: &Link) -> Result<Vec<Call>, FixtureError> {
        match self.action(link, Control::Transcript)? {
            Response::Transcript(calls) => Ok(calls),
            Response::Context(_) | Response::Call(_) | Response::Event(_) | Response::Done => {
                Err(FixtureError::Protocol)
            }
        }
    }
    fn done(
        &mut self,
        link: &Link,
        make: impl FnOnce(Port) -> Control,
    ) -> Result<(), FixtureError> {
        match self.action(link, make)? {
            Response::Done => Ok(()),
            Response::Context(_)
            | Response::Call(_)
            | Response::Event(_)
            | Response::Transcript(_) => Err(FixtureError::Protocol),
        }
    }
    /// Close and reap the actual fixture owner; Drop never claims a close ACK.
    ///
    /// # Errors
    /// Controlled cleanup or the borrowed owner join failed.
    pub fn close(&mut self, link: &Link) -> Result<(), FixtureError> {
        self.done(link, Control::Stop)?;
        if let Some(task) = self.task.as_mut() {
            owned_runtime(link).block_on(async {
                failure::phase(Phase::Close, task)
                    .await?
                    .map_err(|_raw_worker| FixtureError::Worker)?
            })?;
        }
        let _joined = self.task.take();
        Ok(())
    }
}
impl Drop for Scenario {
    fn drop(&mut self) {
        let _previous = self.stop.send_replace(Lifetime::Stopped);
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}
fn outside_runtime() -> Result<(), FixtureError> {
    if tokio::runtime::Handle::try_current().is_ok() {
        Err(FixtureError::NestedRuntime)
    } else {
        Ok(())
    }
}

impl std::fmt::Debug for Scenario {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Scenario(<isolated>)")
    }
}

// The product Link retains its existing keep-alive field and Drop ordering.
// Only the explicit scenario borrows that owner; no runtime/handle is exported.
fn owned_runtime(link: &Link) -> &tokio::runtime::Runtime {
    let Link {
        _runtime: runtime, ..
    } = link;
    runtime
}
