//! The sole command owner; dropping a reply future cannot cancel its worker.
use super::endpoint::{Endpoint, Lifetime};
use super::worker::Worker;
use super::{ComparisonInput, ComparisonPlan, SelectionIndex, Snapshot};
use crate::NodeClient;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, watch};
use vessel_proto::NodeId;

/// Why the client job could not accept a command; no remote retry is implied.
#[derive(Debug, thiserror::Error)]
pub enum JobError {
    /// One queued command already occupies the bounded slot.
    #[error("comparison already has a queued command")]
    Busy,
    /// Owner cancellation or worker termination.
    #[error("comparison is closed")]
    Closed,
    /// The worker ended before acknowledging an admitted command.
    #[error("comparison worker stopped before replying")]
    Response(#[source] oneshot::error::RecvError),
    /// The worker ended before publishing its cancellation checkpoint.
    #[error("comparison worker stopped before its final report")]
    Reports(#[source] watch::error::RecvError),
    /// An immutable common input was already bound.
    #[error("comparison input is already bound")]
    AlreadyBound,
    /// Input position is not owned by this job.
    #[error("selection is outside this comparison")]
    Selection,
    /// No executable waiting/approval state remains for this target.
    #[error("selection cannot be continued; inspect its reported resources")]
    CannotContinue,
    /// Preparation must precede initial input binding.
    #[error("prepare the comparison first")]
    Unprepared,
}
pub(super) enum Action {
    Prepare,
    Start(ComparisonInput),
    Continue(SelectionIndex),
}
pub(super) struct Command {
    pub action: Action,
    pub reply: oneshot::Sender<Result<Arc<Snapshot>, JobError>>,
}

/// Nonclone capability owning preparation, common input and exact continuations.
/// Dropping it cancels further calls; snapshots remain passive recovery reports.
///
/// ```compile_fail
/// use vessel_client::comparison::ComparisonJob;
/// fn duplicate(job: &ComparisonJob) -> ComparisonJob { job.clone() }
/// ```
#[derive(Debug)]
pub struct ComparisonJob {
    command: mpsc::Sender<Command>,
    cancel: watch::Sender<Lifetime>,
    snapshot: watch::Receiver<Arc<Snapshot>>,
}
impl NodeClient {
    /// Construct a comparison owner without sending any remote request.
    /// The caller must retain it through pending setup and partial sends.
    #[must_use]
    pub fn comparison(&self, destination: NodeId, plan: ComparisonPlan) -> ComparisonJob {
        let (command, receiver) = mpsc::channel(1);
        let (cancel, cancellation) = watch::channel(Lifetime::Running);
        let endpoint = Endpoint {
            client: self.clone(),
            destination,
            cancel: cancellation,
        };
        let (worker, snapshot) = Worker::new(endpoint, plan);
        drop(tokio::spawn(worker.run(receiver)));
        ComparisonJob {
            command,
            cancel,
            snapshot,
        }
    }
}
impl ComparisonJob {
    /// Latest passive bounded report; never a replacement command owner.
    #[must_use]
    pub fn snapshot(&self) -> Arc<Snapshot> {
        self.snapshot.borrow().clone()
    }

    /// Observe eventual completion even if a command's reply future was dropped.
    #[must_use]
    pub fn updates(&self) -> watch::Receiver<Arc<Snapshot>> {
        self.snapshot.clone()
    }

    /// Admit initial preparation once; subsequent calls only return its checkpoint.
    ///
    /// # Errors
    /// The command slot is full or this owner is closed.
    pub async fn prepare(&self) -> Result<Arc<Snapshot>, JobError> {
        self.request(Action::Prepare).await
    }
    /// Validate and bind one immutable common input, then send Ready targets.
    ///
    /// # Errors
    /// Already bound, unprepared, command slot full, or owner closed.
    pub async fn start(&self, input: ComparisonInput) -> Result<Arc<Snapshot>, JobError> {
        self.request(Action::Start(input)).await
    }
    /// Continue this owner's pending state, including an externally approved setup.
    /// No-progress eligibility is retained across calls; terminal uncertainty refuses.
    ///
    /// # Errors
    /// Unknown/noncontinuable target, command slot full, or owner closed.
    pub async fn continue_target(&self, index: SelectionIndex) -> Result<Arc<Snapshot>, JobError> {
        self.request(Action::Continue(index)).await
    }
    /// Prevent further phases and wait for the final recovery report.
    /// Already Started targets are retained and no resources are deleted.
    ///
    /// # Errors
    /// The worker ended without a final report.
    pub async fn cancel(&self) -> Result<Arc<Snapshot>, JobError> {
        let _previous = self.cancel.send_replace(Lifetime::Cancelled);
        let mut reports = self.snapshot.clone();
        loop {
            let current = reports.borrow_and_update().clone();
            if current.status == super::JobStatus::Cancelled {
                return Ok(current);
            }
            reports
                .changed()
                .await
                .map_err(|_closed| JobError::Closed)?;
        }
    }
    async fn request(&self, action: Action) -> Result<Arc<Snapshot>, JobError> {
        if *self.cancel.borrow() == Lifetime::Cancelled {
            return Err(JobError::Closed);
        }
        let (reply, receive) = oneshot::channel();
        self.command
            .try_send(Command { action, reply })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => JobError::Busy,
                mpsc::error::TrySendError::Closed(_) => JobError::Closed,
            })?;
        receive.await.map_err(JobError::Response)?
    }
}
impl Drop for ComparisonJob {
    fn drop(&mut self) {
        let _previous = self.cancel.send_replace(Lifetime::Cancelled);
    }
}
