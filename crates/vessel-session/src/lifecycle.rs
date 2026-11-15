//! What every session actor does the same way: publish while staying
//! stoppable, and stop its process in two steps.
//!
//! The actors differ in what they read (PTY bytes, ACP outputs) and how they
//! decide the final status; both keep their process and a [`Lifecycle`],
//! and are generic over [`Stoppable`] (two implementations: the PTY program
//! and the ACP agent), so there is no dynamic dispatch.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, watch};
use tokio::time::Instant;
use vessel_agent::AgentProcess;
use vessel_proto::{
    AgentEvent, ContentBlock, ModelCall, SessionEvent, SessionStatus, SessionTitle,
};
use vessel_pty::{PtyProcess, Signal};

use crate::forge::ForgeView;
use crate::handle::Rename;
use crate::publish::Publisher;
use crate::stop::{Step, Stopper};

/// How long output may keep arriving after the program exited.
///
/// Normally the output ends right after the exit (the group is swept and
/// nothing holds the PTY or pipes any more). A process that left the group
/// (for example a daemon that called `setsid`) can keep them open forever;
/// the session must still end, so draining gives up after this long.
pub(crate) const DRAIN: Duration = Duration::from_secs(2);

/// The deadline for draining, starting now.
pub(crate) fn drain_deadline() -> Instant {
    Instant::now()
        .checked_add(DRAIN)
        .unwrap_or_else(Instant::now)
}

/// A process a session actor stops in two steps.
pub(crate) trait Stoppable {
    /// Carry out `step` (log, do not fail: the actor keeps going).
    fn stop(&mut self, step: Step);
}

impl Stoppable for PtyProcess {
    /// `SIGHUP` first, which is what a closing terminal sends, then
    /// `SIGKILL` for the whole group.
    fn stop(&mut self, step: Step) {
        let signal = match step {
            Step::Ask => Signal::Hangup,
            Step::Force => Signal::Kill,
        };
        if let Err(error) = self.signal(signal) {
            tracing::warn!(pid = self.pid(), %error, ?signal, "cannot signal terminal program");
        }
    }
}

impl Stoppable for AgentProcess {
    /// `SIGTERM` first, then `SIGKILL` for the whole group.
    fn stop(&mut self, step: Step) {
        let signal = match step {
            Step::Ask => Signal::Terminate,
            Step::Force => Signal::Kill,
        };
        if let Err(error) = self.signal(signal) {
            tracing::warn!(pid = self.pid(), %error, ?signal, "cannot signal agent");
        }
    }
}

/// The publisher and the stop state of one session.
#[derive(Debug)]
pub(crate) struct Lifecycle {
    pub(crate) out: Publisher,
    pub(crate) stop: Stopper,
    /// Vessel's own lines for the log, published right after `Starting`
    /// (`LaunchOptions::notices`).
    pub(crate) notices: Vec<String>,
    /// Model calls to record (`SessionHandle::model_call`); the shared
    /// state holds the sender, so this stays open while the actor runs.
    pub(crate) model_calls: mpsc::Receiver<ModelCall>,
    /// Renames to publish (`SessionHandle::rename`, protocol 9); open while
    /// the actor runs, as `model_calls` is.
    pub(crate) renames: mpsc::Receiver<Rename>,
    /// The node's view of the session's pull request (protocol 11,
    /// `SessionHandle::set_pull_request`); the shared state holds the
    /// sender, so it stays open while the actor runs.
    pub(crate) forge: watch::Receiver<Option<Arc<ForgeView>>>,
    /// Whether the session has a title (given, renamed, or taken from a
    /// prompt): only an untitled session takes one from its prompt.
    pub(crate) titled: bool,
}

impl Lifecycle {
    /// The first records: `Status{Starting}`, then the notices, each a
    /// `HarnessLog` line.
    pub(crate) async fn starting<P: Stoppable>(&mut self, process: &mut P) {
        self.status(process, SessionStatus::Starting).await;
        let lines = std::mem::take(&mut self.notices);
        self.notices(process, lines).await;
    }

    /// Publish Vessel's own lines, each a `HarnessLog` record.
    pub(crate) async fn notices<P: Stoppable>(&mut self, process: &mut P, lines: Vec<String>) {
        for line in lines {
            self.publish(
                process,
                SessionEvent::Agent {
                    event: AgentEvent::HarnessLog { line },
                },
            )
            .await;
        }
    }

    /// Publish one event, staying responsive to `kill` while the durable
    /// channel is full.
    pub(crate) async fn publish<P: Stoppable>(&mut self, process: &mut P, event: SessionEvent) {
        let slot = loop {
            tokio::select! {
                biased;
                step = self.stop.next() => self.apply_stop(process, step).await,
                slot = self.out.slot() => break slot,
            }
        };
        self.out.commit(slot, event);
    }

    /// The node gave a new view of the session's pull request.
    pub(crate) fn forge_changed(&mut self) {
        let view = self.forge.borrow_and_update().clone();
        self.out.forge(view.as_deref());
    }

    /// Record the model calls still queued, once the process has exited and
    /// before the final status: a call the proxy finished just as the
    /// harness exited is still the session's.
    pub(crate) async fn drain_model_calls<P: Stoppable>(&mut self, process: &mut P) {
        while let Ok(call) = self.model_calls.try_recv() {
            self.publish(process, SessionEvent::ModelCall(call)).await;
        }
    }

    /// Publish a rename and answer it.
    pub(crate) async fn renamed<P: Stoppable>(&mut self, process: &mut P, rename: Rename) {
        let selected = {
            let current = self.out.current();
            rename.change.resolve(&current)
        };
        let outcome = match selected {
            Some((title, origin)) => {
                let before = self.out.next_seq();
                self.rename(process, title, origin).await;
                if self.out.next_seq() == before {
                    crate::TitleUpdate::Kept
                } else {
                    crate::TitleUpdate::Applied
                }
            }
            None => crate::TitleUpdate::Kept,
        };
        // A cancelled caller does not roll back a committed title.
        let _gone = rename.done.send(outcome);
    }

    /// The title an untitled session takes from a prompt: the first text
    /// block that gives one (`SessionTitle::from_prompt`). So the first
    /// prompt with text names it; one of only images or links does not.
    /// Read before the prompt is published; the title follows it.
    pub(crate) fn title_from(&self, content: &[ContentBlock]) -> Option<SessionTitle> {
        if self.titled {
            return None;
        }
        content.iter().find_map(|block| match block {
            ContentBlock::Text { text } => SessionTitle::from_prompt(text),
            ContentBlock::ResourceLink { .. }
            | ContentBlock::Opaque { .. }
            | ContentBlock::Attachment { .. } => None,
        })
    }

    /// Publish `title` as the session's title.
    pub(crate) async fn rename<P: Stoppable>(
        &mut self,
        process: &mut P,
        title: SessionTitle,
        origin: vessel_proto::TitleOrigin,
    ) {
        self.titled = true;
        self.publish(process, SessionEvent::Renamed { title, origin })
            .await;
    }

    /// Publish a status change.
    pub(crate) async fn status<P: Stoppable>(&mut self, process: &mut P, status: SessionStatus) {
        self.publish(process, SessionEvent::Status { status }).await;
    }

    /// A clean shutdown captures intent before signalling the provider; a
    /// person's kill never captures a continuation. Failure stops the session
    /// anyway and is reported rather than pretending recovery was recorded.
    pub(crate) async fn apply_stop<P: Stoppable>(&mut self, process: &mut P, step: Step) {
        if self.stop.is_idle()
            && self.stop.interrupted()
            && let Err(error) = self.out.capture_restart().await
        {
            tracing::error!(%error, "cannot commit the shutdown recovery boundary");
        }
        self.apply(process, step);
    }

    /// Carry out a stop step and remember it.
    pub(crate) fn apply<P: Stoppable>(&mut self, process: &mut P, step: Step) {
        process.stop(step);
        self.stop.advance(step);
    }

    /// Start stopping without a `kill` (the session cannot continue), unless
    /// a stop is already under way.
    pub(crate) fn begin_stop<P: Stoppable>(&mut self, process: &mut P) {
        if self.stop.is_idle() {
            self.apply(process, Step::Ask);
        }
    }
}
