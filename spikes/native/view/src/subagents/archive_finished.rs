//! Sequential archive progress is view state; A2 owns atomic finished-child admission.
use super::{Target, Track, TrackProblem};
use crate::devices::{Ticket, Tickets};
use std::collections::VecDeque;
use vessel_proto::SessionRef;

/// The pending conditional operation carries both original full identities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishRead {
    /// Expected current managed parent, distinct from detach confirmation.
    pub parent: SessionRef,
    /// Managed child on its original authenticated connection.
    pub child: Target,
    /// Exact operation within the owner's navigation lifetime.
    pub ticket: Ticket,
}
/// Same user-facing states as Paseo's archive-finished controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FinishStatus {
    /// No operation is pending; retained outcomes are available separately.
    #[default]
    Idle,
    /// Processed count includes skipped/failed steps and locally hidden providers.
    Running { completed: usize, total: usize },
    /// Only still-eligible failed managed rows may be retried explicitly.
    Failed {
        /// Failures still eligible after current metadata reconciliation.
        failed: usize,
    },
}
/// This controller never issues a wire call or derives finished eligibility.
#[derive(Debug, Default)]
pub struct ArchiveFinished {
    parent: Option<Target>,
    queue: VecDeque<Target>,
    active: Option<FinishRead>,
    failed: Vec<(Target, TrackProblem)>,
    processed: usize,
    total: usize,
    status: FinishStatus,
}
impl ArchiveFinished {
    /// Stable display state, with no parsing of node/provider error strings.
    #[must_use]
    pub const fn status(&self) -> FinishStatus {
        self.status
    }
    /// Exact pending request for stale-reply and painted-control guards.
    #[must_use]
    pub fn active(&self) -> Option<&FinishRead> {
        self.active.as_ref()
    }
    /// Failure receipts remain attributed to the original child identities.
    #[must_use]
    pub fn failures(&self) -> &[(Target, TrackProblem)] {
        &self.failed
    }
    /// Snapshot canonical A2 eligibility, hide terminal providers locally first.
    /// # Errors
    /// A previous batch is running; a failed batch requires explicit retry.
    pub fn start(&mut self, track: &mut Track) -> Result<(), TrackProblem> {
        if !matches!(self.status, FinishStatus::Idle) {
            return Err(TrackProblem::Busy);
        }
        self.parent = Some(track.target().clone());
        self.queue = track
            .managed()
            .iter()
            .filter(|row| row.finished == Some(true))
            .map(|row| row.target.clone())
            .collect();
        self.failed.clear();
        let before = track.providers(None).count();
        track.hide_finished_providers();
        self.processed = before.saturating_sub(track.providers(None).count());
        self.total = self.processed.saturating_add(self.queue.len());
        self.update(track);
        Ok(())
    }
    /// Select one next item, rechecking the latest hydrated view before dispatch.
    /// Node A2 must still recheck parent+idle/archive atomically under its lock.
    /// # Errors
    /// Another request is outstanding, the parent changed, or tickets exhausted.
    pub fn next(
        &mut self,
        track: &Track,
        tickets: &mut Tickets,
    ) -> Result<Option<FinishRead>, TrackProblem> {
        if self.parent.as_ref() != Some(track.target()) {
            return Err(TrackProblem::IdentityChanged);
        }
        if self.active.is_some() {
            return Err(TrackProblem::Busy);
        }
        while let Some(child) = self.queue.front().cloned() {
            if !eligible(track, child.address) {
                let _skipped = self.queue.pop_front();
                self.processed = self.processed.saturating_add(1);
                continue;
            }
            let ticket = tickets
                .next_ticket()
                .map_err(|_exhausted| TrackProblem::Exhausted)?;
            let _selected = self.queue.pop_front();
            let read = FinishRead {
                parent: track.target().address,
                child,
                ticket,
            };
            self.active = Some(read.clone());
            self.update(track);
            return Ok(Some(read));
        }
        self.update(track);
        Ok(None)
    }
    /// Archived and conditional skip both finish a step; only actual errors retry.
    /// A2's canonical receipt remains with the app caller, not copied into a DTO here.
    pub fn received(
        &mut self,
        read: &FinishRead,
        result: Result<(), TrackProblem>,
        track: &Track,
    ) -> bool {
        if self.active.as_ref() != Some(read) || self.parent.as_ref() != Some(track.target()) {
            return false;
        }
        self.active = None;
        self.processed = self.processed.saturating_add(1);
        if let Err(problem) = result {
            self.failed.push((read.child.clone(), problem));
        }
        self.update(track);
        true
    }
    /// Fresh correlated metadata can remove failures no longer eligible to retry.
    pub fn reconcile(&mut self, track: &Track) {
        if self.parent.as_ref() == Some(track.target()) {
            self.update(track);
        }
    }
    /// Retry only failed current rows, never an already acknowledged success.
    /// # Errors
    /// No retryable batch, changed admission/parent, or a still-running operation.
    pub fn retry(&mut self, track: &Track) -> Result<(), TrackProblem> {
        if self.parent.as_ref() != Some(track.target()) {
            return Err(TrackProblem::IdentityChanged);
        }
        if !matches!(self.status, FinishStatus::Failed { .. }) {
            return Err(TrackProblem::Busy);
        }
        self.queue = self
            .failed
            .iter()
            .filter_map(|(failed, _)| {
                track
                    .managed()
                    .iter()
                    .find(|row| row.target.address == failed.address && row.finished == Some(true))
                    .map(|row| row.target.clone())
            })
            .collect();
        self.failed.clear();
        self.processed = 0;
        self.total = self.queue.len();
        self.update(track);
        Ok(())
    }
    fn update(&mut self, track: &Track) {
        if self.active.is_some() || !self.queue.is_empty() {
            self.status = FinishStatus::Running {
                completed: self.processed,
                total: self.total,
            };
            return;
        }
        self.failed
            .retain(|(child, _)| eligible(track, child.address));
        self.status = if self.failed.is_empty() {
            FinishStatus::Idle
        } else {
            FinishStatus::Failed {
                failed: self.failed.len(),
            }
        };
    }
}
fn eligible(track: &Track, child: SessionRef) -> bool {
    track
        .managed()
        .iter()
        .any(|row| row.target.address == child && row.finished == Some(true))
}
#[cfg(test)]
mod tests;
