//! Admission/fencing skeleton independent of filesystem completion and storage latency.

use super::quota::{Quota, ReservedBytes};
use std::collections::BTreeMap;
use tokio::{sync::oneshot, time::Instant};
use vessel_proto::{
    WorktreeId,
    checkpoint::{CHECKPOINT_WORKERS, CaptureId, CaptureKey, CheckpointError},
};

#[derive(Debug)]
struct Job {
    key: CaptureKey,
    deadline: Instant,
    bytes: ReservedBytes,
    reply: Option<oneshot::Sender<Result<CaptureKey, CheckpointError>>>,
}
/// One owner, used only by the coordinator task; workers never borrow it.
#[derive(Debug, Default)]
pub(super) struct Captures {
    jobs: BTreeMap<CaptureId, Job>,
    quota: Quota,
}
impl Captures {
    pub(super) fn admit(
        &mut self,
        key: CaptureKey,
        deadline: Instant,
        bytes: ReservedBytes,
        reply: oneshot::Sender<Result<CaptureKey, CheckpointError>>,
    ) -> Result<(), CheckpointError> {
        if self.jobs.len() >= CHECKPOINT_WORKERS
            || self.jobs.contains_key(&key.capture)
            || self
                .jobs
                .values()
                .any(|job| job.key.worktree == key.worktree)
        {
            return Err(CheckpointError::Busy);
        }
        self.quota.reserve(bytes)?;
        let previous = self.jobs.insert(
            key.capture,
            Job {
                key,
                deadline,
                bytes,
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
    /// Called only after durable publication acknowledgement; rechecks the lease.
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
        let _closed = reply.send(Ok(key));
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
    pub(super) fn occupied(&self, root: WorktreeId) -> bool {
        self.jobs.values().any(|job| job.key.worktree == root)
    }
    pub(super) fn reserved(&self) -> u64 {
        self.quota.reserved()
    }
}
