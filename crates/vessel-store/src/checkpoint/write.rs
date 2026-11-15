//! One closed command family for the existing store writer; no second connection.
use super::{
    journal,
    paths::{self, Phase},
    publication,
    records::{CheckpointStoreError as Error, Operation, Plan},
};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use vessel_proto::checkpoint::{
    CaptureKey, CheckpointError, CheckpointId, CheckpointPath, ObservedSnapshot, RestoreId,
    RestoreOutcome,
};

#[derive(Debug)]
pub(crate) enum CheckpointWrite {
    EndedMetadata {
        operation: Box<Operation>,
        at: DateTime<Utc>,
    },
    ReserveCapture {
        key: CaptureKey,
        at: DateTime<Utc>,
    },
    CaptureSettled {
        key: CaptureKey,
    },
    // Small accounting models are test-only; real filesystem capture has a
    // fixed escrow, so no production caller can reserve zero for a full image.
    #[cfg(test)]
    Reserve {
        key: CaptureKey,
        bytes: crate::ByteCount,
        at: DateTime<Utc>,
    },
    Candidate {
        key: CaptureKey,
        snapshot: Box<ObservedSnapshot>,
    },
    Activate {
        key: CaptureKey,
        checkpoint: CheckpointId,
    },
    Unavailable {
        key: CaptureKey,
        reason: CheckpointError,
    },
    Plan {
        plan: Box<Plan>,
        at: DateTime<Utc>,
    },
    Admit {
        plan: Box<Plan>,
        restore: RestoreId,
        at: DateTime<Utc>,
    },
    Advance {
        operation: Box<Operation>,
        path: CheckpointPath,
        from: Phase,
        to: Phase,
    },
    Outcome {
        operation: Box<Operation>,
        result: RestoreOutcome,
    },
    Recovered {
        operation: Box<Operation>,
        observed: ObservedSnapshot,
        reason: CheckpointError,
    },
    Allocate {
        key: CaptureKey,
        allocation: super::allocation::Allocation,
        owner: vessel_proto::checkpoint::CheckpointOwner,
        at: DateTime<Utc>,
    },
    AllocationAttached {
        key: CaptureKey,
        allocation: super::allocation::Allocation,
        owner: vessel_proto::checkpoint::CheckpointOwner,
    },
    AllocationCleaned {
        key: CaptureKey,
        allocation: super::allocation::Allocation,
        owner: vessel_proto::checkpoint::CheckpointOwner,
    },
    InvalidateCandidates,
    GarbageCleaned {
        object: super::GarbageObject,
    },
}

/// The writer's savepoint encloses an admission AND every Pending path intent.
pub(crate) fn apply(
    connection: &Connection,
    write: CheckpointWrite,
) -> Result<(), crate::StoreError> {
    let result = match write {
        CheckpointWrite::EndedMetadata { operation, at } => {
            return super::metadata::append_ended(connection, &operation, at);
        }
        CheckpointWrite::ReserveCapture { key, at } => {
            journal::candidate(connection, key, super::quota::CAPTURE_BYTES, at)
        }
        CheckpointWrite::CaptureSettled { key } => super::quota::capture_settled(connection, key),
        #[cfg(test)]
        CheckpointWrite::Reserve { key, bytes, at } => {
            journal::candidate(connection, key, bytes, at)
        }
        CheckpointWrite::Candidate { key, snapshot } => {
            journal::keep_candidate(connection, key, &snapshot)
        }
        CheckpointWrite::Activate { key, checkpoint } => {
            publication::activate(connection, key, checkpoint)
        }
        CheckpointWrite::Unavailable { key, reason } => {
            journal::unavailable(connection, key, reason)
        }
        CheckpointWrite::Plan { plan, at } => journal::put_plan(connection, &plan, at),
        CheckpointWrite::Admit { plan, restore, at } => {
            let new = match journal::status(connection, plan.key.session, restore, plan.owner) {
                Ok(_existing) => false,
                Err(Error::Refused(CheckpointError::NotFound)) => true,
                Err(error) => return Err(error.into()),
            };
            let operation = journal::admit(connection, &plan, restore, at)?;
            if new {
                paths::prepare(connection, &operation)?;
            }
            Ok(())
        }
        CheckpointWrite::Advance {
            operation,
            path,
            from,
            to,
        } => paths::advance(connection, &operation, &path, from, to),
        CheckpointWrite::Outcome { operation, result } => {
            journal::outcome(connection, &operation, &result)
        }
        CheckpointWrite::Recovered {
            operation,
            observed,
            reason,
        } => super::recovery::resolved(connection, &operation, &observed, reason),
        CheckpointWrite::Allocate {
            key,
            allocation,
            owner,
            at,
        } => super::allocation::reserve(connection, key, allocation, owner, at),
        CheckpointWrite::AllocationAttached {
            key,
            allocation,
            owner,
        } => super::allocation::attach(connection, key, allocation, owner),
        CheckpointWrite::AllocationCleaned {
            key,
            allocation,
            owner,
        } => super::allocation::cleaned(connection, key, allocation, owner),
        CheckpointWrite::InvalidateCandidates => journal::invalidate_candidates(connection),
        CheckpointWrite::GarbageCleaned { object } => super::garbage::cleaned(connection, object),
    };
    result.map_err(crate::StoreError::from)
}
