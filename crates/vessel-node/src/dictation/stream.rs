//! One exclusive stream owner: data progress never gates control or update reads.

use super::{
    audio::{Resampler, Segment, SegmentBuffer},
    commands::{Current, Data, Handle},
    control::Control,
    input::Input,
    jobs::{CommitPhase, Completion, JobError, Jobs, Source},
    stream_error::{StreamError, StreamResult},
    text::Text,
    updates::{Delivery, Reply, Updates},
};
use std::{
    future::{Future, pending},
    pin::Pin,
    sync::Arc,
    time::Duration,
};
use tokio::{
    sync::{mpsc, oneshot, watch},
    task::JoinHandle,
    time::{Instant, sleep_until},
};
use tokio_util::sync::CancellationToken;
use vessel_proto::dictation::{SampleRate, WithinMs};

mod progress;
mod transitions;
const DATA_MAILBOX: usize = 1;
const NEXT_MAILBOX: usize = 1;

type Waiter = oneshot::Sender<Reply<Arc<StreamError>>>;

pub(super) struct CloseSignals {
    pub(super) owner: CancellationToken,
    pub(super) node: CancellationToken,
}
struct Audio {
    resampler: Resampler,
    buffer: SegmentBuffer,
}
enum Seal {
    Open,
    Waiting(Segment),
    Sealed,
}
struct Finishing {
    accepted: Option<(WithinMs, Instant)>,
    seal: Seal,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Life {
    Live,
    Terminal,
    Consumed,
}

struct Stream {
    input: Option<Input>,
    audio: Option<Audio>,
    text: Text,
    updates: Updates<Arc<StreamError>>,
    jobs: Option<Jobs>,
    finishing: Option<Finishing>,
    current: Option<Current>,
    deferred: Option<Completion>,
    phase: watch::Sender<CommitPhase>,
    control: Control<StreamError>,
    life: Life,
}

pub(super) fn spawn<F>(
    rate: SampleRate,
    source: Source,
    signals: CloseSignals,
    transport: F,
) -> StreamResult<(Handle, JoinHandle<()>)>
where
    F: Future<Output = ()> + Send + 'static,
{
    let input = Input::new(rate).map_err(|source| Arc::new(StreamError::Input(source)))?;
    let buffer = SegmentBuffer::new().map_err(|source| Arc::new(StreamError::Audio(source)))?;
    let control = Control::new(&signals.owner);
    let (phase, phase_rx) = watch::channel(CommitPhase::Running);
    let (data, data_rx) = mpsc::channel(DATA_MAILBOX);
    let (next, next_rx) = mpsc::channel(NEXT_MAILBOX);
    let closed = CancellationToken::new();
    let completed = closed.clone().drop_guard();
    let state = Stream {
        input: Some(input),
        audio: Some(Audio {
            resampler: Resampler::new(rate),
            buffer,
        }),
        text: Text::default(),
        updates: Updates::default(),
        jobs: Some(Jobs::new(source, phase_rx)),
        finishing: None,
        current: None,
        deferred: None,
        phase,
        control: control.clone(),
        life: Life::Live,
    };
    let task = tokio::spawn(async move {
        let _completed = completed;
        state.run(data_rx, next_rx, signals, transport).await;
    });
    Ok((
        Handle {
            data,
            next,
            control,
            closed,
        },
        task,
    ))
}

enum Wake {
    Closed,
    Retire,
    Stop,
    Deadline,
    Data(Option<Data>),
    Next(Option<Waiter>),
    Completed(Option<Result<Completion, JobError>>),
    Progress,
}
impl Stream {
    async fn run<F>(
        mut self,
        mut data: mpsc::Receiver<Data>,
        mut next: mpsc::Receiver<Waiter>,
        signals: CloseSignals,
        transport: F,
    ) where
        F: Future<Output = ()>,
    {
        tokio::pin!(transport);
        while self.life != Life::Consumed {
            if signals.owner.is_cancelled() || signals.node.is_cancelled() {
                self.closed().await;
                break;
            }
            if self.control.retiring().is_cancelled() {
                self.retire().await;
                break;
            }
            if self.life == Life::Live && self.control.work().is_cancelled() {
                let reason = self
                    .control
                    .chosen()
                    .unwrap_or_else(|| Arc::new(StreamError::Cancelled));
                self.fail(reason).await;
            }
            if self.updates.pump() == Delivery::TerminalReady {
                break;
            }
            let event = self
                .select(&mut data, &mut next, &signals, transport.as_mut())
                .await;
            self.apply(event).await;
        }
        self.drain_pending(&mut data, &mut next);
        self.release().await;
    }

    async fn select<F>(
        &mut self,
        data: &mut mpsc::Receiver<Data>,
        next: &mut mpsc::Receiver<Waiter>,
        signals: &CloseSignals,
        transport: Pin<&mut F>,
    ) -> Wake
    where
        F: Future<Output = ()>,
    {
        let work = self.control.work();
        let retire = self.control.retiring();
        let live = self.life == Life::Live;
        let receive_data = self.current.is_none();
        let take_job =
            self.deferred.is_none() && self.jobs.as_ref().is_some_and(|jobs| !jobs.empty());
        let progress = self.can_progress();
        let deadline = self
            .finishing
            .as_ref()
            .and_then(|state| state.accepted.map(|(_, at)| at));
        tokio::select! {
            biased;
            () = retire.cancelled() => Wake::Retire,
            () = signals.owner.cancelled() => Wake::Closed,
            () = signals.node.cancelled() => Wake::Closed,
            () = transport => Wake::Closed,
            () = work.cancelled(), if live => Wake::Stop,
            () = deadline_at(deadline), if live => Wake::Deadline,
            reply = next.recv() => Wake::Next(reply),
            completion = collect(&mut self.jobs), if take_job => Wake::Completed(completion),
            command = data.recv(), if receive_data => Wake::Data(command),
            () = tokio::task::yield_now(), if progress => Wake::Progress,
        }
    }
}
async fn deadline_at(deadline: Option<Instant>) {
    match deadline {
        Some(at) => sleep_until(at).await,
        None => pending().await,
    }
}
async fn collect(jobs: &mut Option<Jobs>) -> Option<Result<Completion, JobError>> {
    match jobs {
        Some(jobs) => jobs.take().await,
        None => pending().await,
    }
}

#[cfg(test)]
mod tests;
