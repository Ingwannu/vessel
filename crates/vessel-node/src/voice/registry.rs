//! One voice reservation per live connection and session, including uncertain cleanup.
//!
//! The caller serializes this short, non-awaiting book around admission and
//! state changes. Provider/reload tasks retain only a generation and child
//! cancellation token, never the connection lifetime guard. A cancelled start
//! cannot later publish Ready or acquire another caller's active reservation.

use std::collections::HashMap;

use tokio_util::sync::CancellationToken;
use vessel_proto::{
    SessionId,
    voice::{
        ReadyVoice, StartVoice, StopVoiceResult, VoiceEpoch, VoiceFailure, VoiceId, VoiceLease,
        VoiceStopTarget,
    },
};

use crate::connection_life::{ConnectionLife, Owner};

const ACTIVE_PER_NODE: usize = 8;
// Original start ids remain bound until that connection has closed AND remote
// teardown is confirmed. These separate caps bound retained nonce history.
const ATTEMPTS_PER_CONNECTION: usize = 128;
const ATTEMPTS_PER_NODE: usize = 1_024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum Refusal {
    #[error("voice ownership does not match")]
    Owner,
    #[error("voice generation is unavailable")]
    Generation,
    #[error("voice start identity was already used")]
    Reused,
    #[error("voice ownership is still reserved")]
    Busy,
    #[error("voice reservation capacity is exhausted")]
    Capacity,
    #[error("voice is stopping or its connection closed")]
    Stopped,
    #[error("voice completion arrived out of order")]
    Order,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Starting,
    Active,
    Stopping,
    Uncertain(VoiceFailure),
    Retired,
}
impl Phase {
    fn reserved(self) -> bool {
        self != Self::Retired
    }
}

#[derive(Debug, Clone)]
pub(super) struct Reservation {
    pub(super) ready: ReadyVoice,
    pub(super) cancel: CancellationToken,
}

#[derive(Debug)]
struct Entry {
    owner: Owner,
    closed: CancellationToken,
    reservation: Reservation,
    phase: Phase,
}

#[derive(Debug)]
pub(super) enum Stop {
    Complete,
    Cleanup(Reservation),
}

#[derive(Debug, Default)]
pub(super) struct Registry {
    entries: HashMap<VoiceId, Entry>,
    cancelled_starts: HashMap<(Owner, VoiceId), CancellationToken>,
}
impl Registry {
    /// Readiness, session authority and reload capability preflight precede this
    /// method. It reserves no provider/device and does not itself enable voice.
    pub(super) fn reserve(
        &mut self,
        connection: &ConnectionLife,
        request: StartVoice,
    ) -> Result<Reservation, Refusal> {
        self.reap();
        let closed = connection.closed();
        if closed.is_cancelled() {
            return Err(Refusal::Stopped);
        }
        let owner = connection.owner();
        self.admit(&owner, request)?;
        let reservation = Reservation {
            ready: ReadyVoice {
                lease: VoiceLease {
                    id: request.id,
                    epoch: VoiceEpoch::random(),
                },
                session: request.session,
            },
            cancel: closed.child_token(),
        };
        let _previous = self.entries.insert(
            request.id,
            Entry {
                owner,
                closed,
                reservation: reservation.clone(),
                phase: Phase::Starting,
            },
        );
        Ok(reservation)
    }
    fn admit(&self, owner: &Owner, request: StartVoice) -> Result<(), Refusal> {
        if self
            .cancelled_starts
            .contains_key(&(owner.clone(), request.id))
        {
            return Err(Refusal::Stopped);
        }
        if self.entries.contains_key(&request.id) {
            return Err(Refusal::Reused);
        }
        if self.entries.values().any(|entry| {
            entry.phase.reserved()
                && (&entry.owner == owner || entry.reservation.ready.session == request.session)
        }) {
            return Err(Refusal::Busy);
        }
        let active = self
            .entries
            .values()
            .filter(|entry| entry.phase.reserved())
            .count();
        if active >= ACTIVE_PER_NODE {
            return Err(Refusal::Capacity);
        }
        self.admit_nonce(owner)?;
        Ok(())
    }
    fn admit_nonce(&self, owner: &Owner) -> Result<(), Refusal> {
        let retained = self
            .entries
            .values()
            .filter(|entry| &entry.owner == owner)
            .count()
            .saturating_add(
                self.cancelled_starts
                    .keys()
                    .filter(|(key, _id)| key == owner)
                    .count(),
            );
        if retained >= ATTEMPTS_PER_CONNECTION
            || self
                .entries
                .len()
                .saturating_add(self.cancelled_starts.len())
                >= ATTEMPTS_PER_NODE
        {
            return Err(Refusal::Capacity);
        }
        Ok(())
    }
    /// Called only after provider and transient reload success for this exact
    /// generation. No late success can reopen a locally cancelled start.
    pub(super) fn activate(&mut self, lease: VoiceLease) -> Result<ReadyVoice, Refusal> {
        let entry = self.entry(lease)?;
        if entry.reservation.cancel.is_cancelled() {
            return Err(Refusal::Stopped);
        }
        if entry.phase != Phase::Starting {
            return Err(Refusal::Order);
        }
        entry.phase = Phase::Active;
        Ok(entry.reservation.ready)
    }
    pub(super) fn authorize(
        &self,
        owner: &Owner,
        lease: VoiceLease,
    ) -> Result<Reservation, Refusal> {
        let entry = self.entries.get(&lease.id).ok_or(Refusal::Generation)?;
        if &entry.owner != owner {
            return Err(Refusal::Owner);
        }
        if entry.reservation.ready.lease != lease {
            return Err(Refusal::Generation);
        }
        if entry.phase != Phase::Active || entry.reservation.cancel.is_cancelled() {
            return Err(Refusal::Stopped);
        }
        Ok(entry.reservation.clone())
    }
    /// Original-nonce Stop also finds a late Ready; that nonce never rebinds.
    /// Cancellation happens here, before any caller awaits remote restoration.
    pub(super) fn stop(
        &mut self,
        connection: &ConnectionLife,
        target: VoiceStopTarget,
    ) -> Result<Stop, Refusal> {
        self.reap();
        let owner = connection.owner();
        let id = match target {
            VoiceStopTarget::Starting { id } => id,
            VoiceStopTarget::Active { lease } => lease.id,
        };
        if !self.entries.contains_key(&id) {
            return match target {
                VoiceStopTarget::Starting { .. } => self.cancel_pending(connection, id),
                VoiceStopTarget::Active { .. } => Err(Refusal::Generation),
            };
        }
        let entry = self.entries.get_mut(&id).ok_or(Refusal::Generation)?;
        if entry.owner != owner {
            return Err(Refusal::Owner);
        }
        if let VoiceStopTarget::Active { lease } = target
            && lease != entry.reservation.ready.lease
        {
            return Err(Refusal::Generation);
        }
        entry.reservation.cancel.cancel();
        if matches!(entry.phase, Phase::Starting | Phase::Active) {
            entry.phase = Phase::Stopping;
        }
        Ok(if entry.phase == Phase::Retired {
            Stop::Complete
        } else {
            Stop::Cleanup(entry.reservation.clone())
        })
    }
    fn cancel_pending(
        &mut self,
        connection: &ConnectionLife,
        id: VoiceId,
    ) -> Result<Stop, Refusal> {
        let key = (connection.owner(), id);
        if !self.cancelled_starts.contains_key(&key) {
            self.admit_nonce(&key.0)?;
            let _previous = self.cancelled_starts.insert(key, connection.closed());
        }
        Ok(Stop::Complete)
    }
    /// A trusted teardown worker reports restoration truthfully. Uncertainty
    /// retains both quota and session exclusion; closure is not teardown proof.
    pub(super) fn finish(
        &mut self,
        lease: VoiceLease,
        outcome: StopVoiceResult,
    ) -> Result<(), Refusal> {
        let entry = self.entry(lease)?;
        if entry.phase == Phase::Retired {
            return Ok(());
        }
        entry.reservation.cancel.cancel();
        entry.phase = match outcome {
            StopVoiceResult::Stopped => Phase::Retired,
            StopVoiceResult::Uncertain { reason } => Phase::Uncertain(reason),
        };
        self.reap();
        Ok(())
    }
    /// The MCP producer separately checks its explicit transient voice context
    /// and authenticated agent session; no caller-supplied machine selector.
    pub(super) fn active_for_session(&self, session: SessionId) -> Option<Reservation> {
        self.entries
            .values()
            .find(|entry| {
                entry.reservation.ready.session == session
                    && entry.phase == Phase::Active
                    && !entry.reservation.cancel.is_cancelled()
            })
            .map(|entry| entry.reservation.clone())
    }
    fn entry(&mut self, lease: VoiceLease) -> Result<&mut Entry, Refusal> {
        self.entries
            .get_mut(&lease.id)
            .filter(|entry| entry.reservation.ready.lease == lease)
            .ok_or(Refusal::Generation)
    }
    fn reap(&mut self) {
        self.entries
            .retain(|_id, entry| entry.phase != Phase::Retired || !entry.closed.is_cancelled());
        self.cancelled_starts
            .retain(|_key, closed| !closed.is_cancelled());
    }
}

#[cfg(test)]
mod tests;
