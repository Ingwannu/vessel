//! The synchronous handoff from one reserved identity to its newly started actor.
//! Refusal retains both the reservation and handle in the node's existing task owner.
use super::{Entry, Plan, Reservation, State, Terminals, lock};
use crate::terminals::{CLOSE_WAIT, MAX_ENDED};
use std::collections::hash_map::Entry as MapEntry;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use vessel_proto::{ErrorCode, TerminalId, WireError};
use vessel_session::terminals::TerminalHandle;

#[derive(Debug, Clone, Copy, thiserror::Error)]
enum Refusal {
    #[error("terminal reservation belongs to another registry instance")]
    ForeignRegistry,
    #[error("terminal {0} no longer has its original start reservation")]
    MissingReservation(TerminalId),
    #[error("terminal registration expected {expected}, received {actual}")]
    WrongId {
        expected: TerminalId,
        actual: TerminalId,
    },
    #[error("terminal {0} is already registered")]
    Duplicate(TerminalId),
}

/// A refused handoff already owns tracked cleanup; dropping its observer cannot cancel it.
#[derive(Debug)]
#[must_use = "observe the refused registration; cleanup remains tracked independently"]
pub(crate) struct RegistrationRejected {
    reason: Refusal,
    cleanup: JoinHandle<()>,
}
impl RegistrationRejected {
    fn retained(
        reason: Refusal,
        reservation: Reservation,
        handle: TerminalHandle,
        plan: Option<Arc<Plan>>,
        target: &Mutex<State>,
    ) -> Self {
        let id = handle.id();
        // Never kill an incumbent merely because an internal caller supplied its
        // handle again. Separate short locks avoid ordering two registry locks.
        let incumbent = contains(target, id) || contains(&reservation.state, id);
        if !incumbent {
            handle.kill();
        }
        let tasks = reservation.tasks.clone();
        let cleanup = tasks.spawn(async move {
            handle.ended().await;
            drop(plan);
            drop(reservation);
        });
        Self { reason, cleanup }
    }

    /// Observe cleanup for the existing close budget; expiry never cancels its owner.
    /// This observes the base actor's end, not descendant or physical-root quiescence.
    pub(crate) async fn into_wire(self) -> WireError {
        let Self { reason, cleanup } = self;
        match tokio::time::timeout(CLOSE_WAIT, cleanup).await {
            Ok(Ok(())) => WireError::new(ErrorCode::Conflict, reason.to_string()),
            Ok(Err(error)) => WireError::new(
                ErrorCode::Internal,
                format!("{reason}; rejected terminal cleanup task failed: {error}"),
            ),
            Err(_elapsed) => WireError::new(
                ErrorCode::Unavailable,
                format!("{reason}; rejected terminal cleanup remains tracked"),
            ),
        }
    }
}

fn contains(state: &Mutex<State>, id: TerminalId) -> bool {
    lock(state).entries.contains_key(&id)
}

impl Terminals {
    /// Bind only this registry's exact, still-held identity before exposing a handle.
    /// Synchronous success preserves registration-before-the-next-await ordering.
    pub(crate) fn register(
        &self,
        reservation: Reservation,
        handle: TerminalHandle,
        plan: Option<Arc<Plan>>,
    ) -> Result<(), RegistrationRejected> {
        let id = handle.id();
        let mut state = self.state();
        let refusal = if !Arc::ptr_eq(&self.state, &reservation.state) {
            Some(Refusal::ForeignRegistry)
        } else if id != reservation.id {
            Some(Refusal::WrongId {
                expected: reservation.id,
                actual: id,
            })
        } else if !state.starting.iter().any(|(identity, start)| {
            Arc::ptr_eq(identity, &reservation.identity)
                && start.id == reservation.id
                && start.owner == reservation.owner
        }) {
            Some(Refusal::MissingReservation(reservation.id))
        } else {
            None
        };
        if let Some(reason) = refusal {
            drop(state);
            return Err(RegistrationRejected::retained(
                reason,
                reservation,
                handle,
                plan,
                &self.state,
            ));
        }
        match state.entries.entry(id) {
            MapEntry::Vacant(slot) => {
                let _inserted = slot.insert(Entry {
                    handle,
                    plan,
                    owner: reservation.owner.clone(),
                });
            }
            MapEntry::Occupied(slot) => {
                drop(slot);
                drop(state);
                return Err(RegistrationRejected::retained(
                    Refusal::Duplicate(id),
                    reservation,
                    handle,
                    plan,
                    &self.state,
                ));
            }
        }
        state
            .starting
            .retain(|(identity, _)| !Arc::ptr_eq(identity, &reservation.identity));
        prune_ended(&mut state);
        drop(state);
        drop(reservation);
        Ok(())
    }
}

/// Preserve the existing bounded ended inventory; no cleanup-debt proof is added here.
fn prune_ended(state: &mut State) {
    let mut ended: Vec<_> = state
        .entries
        .values()
        .filter(|entry| entry.handle.is_ended())
        .map(|entry| {
            let info = entry.handle.info();
            (info.id, info.ended_at.unwrap_or(info.created_at))
        })
        .collect();
    if ended.len() > MAX_ENDED {
        ended.sort_by_key(|(_, at)| *at);
        let excess = ended.len().saturating_sub(MAX_ENDED);
        for (old, _) in ended.into_iter().take(excess) {
            drop(state.entries.remove(&old));
        }
    }
}
