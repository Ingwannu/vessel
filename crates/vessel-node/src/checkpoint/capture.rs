//! Admission/fencing skeleton independent of filesystem completion and storage latency.

use super::{
    quota::{Quota, ReservedBytes},
    workers::{WorkerLease, Workers},
};
use std::{collections::BTreeMap, sync::Arc};
use tokio::{sync::oneshot, time::Instant};
#[cfg(test)]
use vessel_proto::WorktreeId;
use vessel_proto::checkpoint::{
    CHECKPOINT_WORKERS, CaptureId, CaptureKey, CheckpointError, PhysicalRoot,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkerState {
    Running,
    CleanupFailed,
}
#[derive(Debug)]
struct Job {
    key: CaptureKey,
    worker: WorkerState,
    lease: Arc<WorkerLease>,
    physical: PhysicalRoot,
    deadline: Instant,
    bytes: ReservedBytes,
    cancel: tokio_util::sync::CancellationToken,
    reply: Option<oneshot::Sender<Result<CaptureKey, CheckpointError>>>,
}
/// One owner, used only by the coordinator task; workers never borrow it.
#[derive(Debug, Default)]
pub(super) struct Captures {
    jobs: BTreeMap<CaptureId, Job>,
    quota: Quota,
    workers: Workers,
}
impl Captures {
    pub(super) fn new(workers: Workers) -> Self {
        Self {
            workers,
            jobs: BTreeMap::new(),
            quota: Quota::default(),
        }
    }
    pub(super) fn worker(&self, key: CaptureKey) -> Option<Arc<WorkerLease>> {
        self.jobs
            .get(&key.capture)
            .filter(|job| job.key == key)
            .map(|job| Arc::clone(&job.lease))
    }
    /// Failure keeps admission while serving, but cannot hang node shutdown
    /// after every actual I/O job returned. Durable escrow remains for startup.
    pub(super) fn drained(&self) -> bool {
        self.jobs
            .values()
            .all(|job| job.worker == WorkerState::CleanupFailed)
    }
    pub(super) fn cleanup_failed(&mut self, key: CaptureKey, reason: CheckpointError) {
        self.refuse(key, reason);
        if let Some(job) = self.jobs.get_mut(&key.capture)
            && job.key == key
        {
            job.worker = WorkerState::CleanupFailed;
        }
    }
    #[cfg(test)]
    pub(super) fn admit(
        &mut self,
        key: CaptureKey,
        deadline: Instant,
        bytes: ReservedBytes,
        reply: oneshot::Sender<Result<CaptureKey, CheckpointError>>,
    ) -> Result<(), CheckpointError> {
        self.admit_at_root(key, model_root(key.worktree), deadline, bytes, reply)
    }
    pub(super) fn admit_at_root(
        &mut self,
        key: CaptureKey,
        physical: PhysicalRoot,
        deadline: Instant,
        bytes: ReservedBytes,
        reply: oneshot::Sender<Result<CaptureKey, CheckpointError>>,
    ) -> Result<(), CheckpointError> {
        if self.jobs.len() >= CHECKPOINT_WORKERS
            || self.jobs.contains_key(&key.capture)
            || self.jobs.values().any(|job| job.physical == physical)
        {
            let _closed = reply.send(Err(CheckpointError::Busy));
            return Err(CheckpointError::Busy);
        }
        let lease = match self.workers.claim() {
            Ok(lease) => lease,
            Err(error) => {
                let _closed = reply.send(Err(error));
                return Err(error);
            }
        };
        if let Err(error) = self.quota.reserve(bytes) {
            let _closed = reply.send(Err(error));
            return Err(error);
        }
        let previous = self.jobs.insert(
            key.capture,
            Job {
                key,
                worker: WorkerState::Running,
                lease,
                physical,
                deadline,
                bytes,
                cancel: tokio_util::sync::CancellationToken::new(),
                reply: Some(reply),
            },
        );
        debug_assert!(previous.is_none());
        Ok(())
    }
    /// Deadline terminalizes acknowledgement but retains all physical admission.
    pub(super) fn expire(&mut self, now: Instant) {
        for job in self.jobs.values_mut() {
            if now >= job.deadline
                && let Some(reply) = job.reply.take()
            {
                job.cancel.cancel();
                let _closed = reply.send(Err(CheckpointError::CaptureTimedOut));
            }
        }
    }
    /// Only a matching still-live candidate is eligible for durable publication.
    pub(super) fn candidate(
        &mut self,
        key: CaptureKey,
        now: Instant,
    ) -> Result<(), CheckpointError> {
        self.expire(now);
        let job = self
            .jobs
            .get(&key.capture)
            .ok_or(CheckpointError::StaleCapture)?;
        if job.key != key || job.reply.is_none() {
            return Err(CheckpointError::StaleCapture);
        }
        Ok(())
    }
    /// Durable candidate acceptance, not Ready activation; rechecks the lease.
    pub(super) fn published(
        &mut self,
        key: CaptureKey,
        now: Instant,
    ) -> Result<(), CheckpointError> {
        self.candidate(key, now)?;
        let job = self
            .jobs
            .get_mut(&key.capture)
            .ok_or(CheckpointError::StaleCapture)?;
        let reply = job.reply.take().ok_or(CheckpointError::StaleCapture)?;
        if reply.send(Ok(key)).is_err() {
            job.cancel.cancel();
            return Err(CheckpointError::StaleCapture);
        }
        Ok(())
    }
    /// Workers signal this only after their staging/handles have been cleaned up.
    pub(super) fn cleaned(&mut self, key: CaptureKey) -> Result<(), CheckpointError> {
        let job = self
            .jobs
            .get(&key.capture)
            .ok_or(CheckpointError::StaleCapture)?;
        if job.key != key {
            return Err(CheckpointError::StaleCapture);
        }
        let job = self
            .jobs
            .remove(&key.capture)
            .ok_or(CheckpointError::StaleCapture)?;
        self.quota.release(job.bytes);
        Ok(())
    }
    pub(super) fn deadline(&self) -> Option<Instant> {
        self.jobs
            .values()
            .filter(|job| job.reply.is_some())
            .map(|job| job.deadline)
            .min()
    }
    pub(super) fn cancellation(
        &self,
        key: CaptureKey,
    ) -> Option<tokio_util::sync::CancellationToken> {
        self.jobs
            .get(&key.capture)
            .filter(|job| job.key == key)
            .map(|job| job.cancel.clone())
    }
    pub(super) fn refuse(&mut self, key: CaptureKey, reason: CheckpointError) {
        if let Some(job) = self.jobs.get_mut(&key.capture)
            && job.key == key
        {
            job.cancel.cancel();
            if let Some(reply) = job.reply.take() {
                let _closed = reply.send(Err(reason));
            }
        }
    }
    #[cfg(test)]
    pub(super) fn occupied(&self, root: WorktreeId) -> bool {
        self.jobs.values().any(|job| job.key.worktree == root)
    }
    #[cfg(test)]
    pub(super) fn reserved(&self) -> u64 {
        self.quota.reserved()
    }
}

// Pure admission-model fixtures only. Product callers supply an opened-FD root;
// no logical worktree identity is accepted as a physical lock key in production.
#[cfg(test)]
pub(super) fn model_root(worktree: WorktreeId) -> PhysicalRoot {
    use sha2::Digest as _;
    PhysicalRoot::new(vessel_proto::BlobHash::from_bytes(
        sha2::Sha256::digest(worktree.to_string().as_bytes()).into(),
    ))
}
