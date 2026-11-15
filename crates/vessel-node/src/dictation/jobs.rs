//! Bounded independent provider jobs and their completion-time phase.
//!
//! A later job may complete before an earlier one. The job captures finish phase
//! before output backpressure can delay collection; it never owns a connection guard.

use super::{
    audio::Segment,
    endpoint::{Endpoint, ProviderError},
    text::CommitIndex,
};
#[cfg(test)]
use tokio::sync::{mpsc, oneshot};
use tokio::{sync::watch, task::JoinSet};
use vessel_proto::dictation::Transcript;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommitPhase {
    Running,
    Finishing,
}

#[derive(Clone)]
enum Source {
    Endpoint(Endpoint),
    #[cfg(test)]
    Fake(mpsc::Sender<FakeRequest>),
}
#[cfg(test)]
struct FakeRequest {
    segment: Segment,
    reply: oneshot::Sender<Result<Transcript, ProviderError>>,
}

#[derive(thiserror::Error)]
enum JobError {
    #[error("the dictation provider did not complete the segment")]
    Endpoint(#[source] ProviderError),
    #[error("the dictation provider worker did not finish")]
    Join(#[source] tokio::task::JoinError),
    #[cfg(test)]
    #[error("the synthetic provider closed before accepting work")]
    Send(#[source] mpsc::error::SendError<()>),
    #[cfg(test)]
    #[error("the synthetic provider closed without a result")]
    Reply(#[source] oneshot::error::RecvError),
}
impl std::fmt::Debug for JobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
impl Source {
    async fn transcribe(&self, segment: &Segment) -> Result<Transcript, JobError> {
        match self {
            Self::Endpoint(endpoint) => endpoint
                .transcribe(segment)
                .await
                .map_err(JobError::Endpoint),
            #[cfg(test)]
            Self::Fake(sender) => {
                let slot = sender.reserve().await.map_err(JobError::Send)?;
                let (reply, result) = oneshot::channel();
                slot.send(FakeRequest {
                    segment: segment.clone(),
                    reply,
                });
                result
                    .await
                    .map_err(JobError::Reply)?
                    .map_err(JobError::Endpoint)
            }
        }
    }
}

struct Completion {
    index: CommitIndex,
    phase: CommitPhase,
    result: Result<Transcript, JobError>,
}

#[derive(Debug, thiserror::Error)]
#[error("dictation already holds {max} provider job slots", max = super::MAX_IN_FLIGHT_COMMITS)]
struct Full {
    index: CommitIndex,
    segment: Segment,
}

struct Jobs {
    source: Source,
    phase: watch::Receiver<CommitPhase>,
    tasks: JoinSet<Completion>,
    #[cfg(test)]
    observed: Option<watch::Sender<Option<(CommitIndex, CommitPhase)>>>,
}
impl Jobs {
    fn new(source: Source, phase: watch::Receiver<CommitPhase>) -> Self {
        Self {
            source,
            phase,
            tasks: JoinSet::new(),
            #[cfg(test)]
            observed: None,
        }
    }
    fn full(&self) -> bool {
        self.tasks.len() >= super::MAX_IN_FLIGHT_COMMITS
    }
    fn empty(&self) -> bool {
        self.tasks.is_empty()
    }
    fn spawn(&mut self, index: CommitIndex, segment: Segment) -> Result<(), Full> {
        if self.full() {
            return Err(Full { index, segment });
        }
        let source = self.source.clone();
        let phase = self.phase.clone();
        #[cfg(test)]
        let observed = self.observed.clone();
        let _abort = self.tasks.spawn(async move {
            let result = source.transcribe(&segment).await;
            // No await separates the provider return from this phase observation.
            let phase = *phase.borrow();
            #[cfg(test)]
            if let Some(observed) = observed {
                let _previous = observed.send_replace(Some((index, phase)));
            }
            Completion {
                index,
                phase,
                result,
            }
        });
        Ok(())
    }
    async fn take(&mut self) -> Option<Result<Completion, JobError>> {
        self.tasks
            .join_next()
            .await
            .map(|result| result.map_err(JobError::Join))
    }
    async fn clear(&mut self) -> Result<(), JobError> {
        self.tasks.abort_all();
        let mut failed = None;
        while let Some(result) = self.tasks.join_next().await {
            match result {
                Ok(_) => {}
                Err(error) if error.is_cancelled() => {}
                Err(error) => {
                    if failed.is_none() {
                        failed = Some(JobError::Join(error));
                    }
                }
            }
        }
        failed.map_or(Ok(()), Err)
    }
}

#[cfg(test)]
#[path = "jobs_tests.rs"]
mod tests;
