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
/// Typed commands for the node's checkpoint coordinator, using the sole writer.
/// This API is not an authorization boundary: the node authenticates principals,
/// owns physical admission and validates actual I/O/postconditions before calling.
pub enum CheckpointWrite {
    /// Record a rejected boundary without pretending any filesystem worker ran.
    RefusedBoundary {
        /// Exact persisted anchor and non-reusable attempt identity.
        key: CaptureKey,
        /// Redacted stable refusal for the turn marker.
        reason: CheckpointError,
        /// Coordinator observation time.
        at: DateTime<Utc>,
    },
    /// Retire a bounded batch of old unpinned images without releasing GC bytes.
    Collect {
        /// Coordinator observation time for preview pin/retention policy.
        at: DateTime<Utc>,
    },
    /// Append a successful result outside a run; repeat reads the stored event marker.
    EndedMetadata {
        /// Exact owned durable operation snapshot.
        operation: Box<Operation>,
        /// Coordinator observation time.
        at: DateTime<Utc>,
    },
    /// Reserve the fixed capture escrow before any filesystem read.
    ReserveCapture {
        /// Immutable capture boundary and generation.
        key: CaptureKey,
        /// Coordinator observation time.
        at: DateTime<Utc>,
    },
    /// Settle a terminal capture only after its actual worker cleanup finishes.
    CaptureSettled {
        /// Immutable capture boundary and generation.
        key: CaptureKey,
    },
    // Small accounting models are test-only; real filesystem capture has a
    // fixed escrow, so no production caller can reserve zero for a full image.
    #[cfg(test)]
    /// Test-only small accounting model; never a production filesystem escrow.
    Reserve {
        /// Immutable capture boundary and generation.
        key: CaptureKey,
        /// Test accounting reservation.
        bytes: crate::ByteCount,
        /// Coordinator observation time.
        at: DateTime<Utc>,
    },
    /// Persist immutable evidence without making it Ready.
    Candidate {
        /// Immutable capture boundary and generation.
        key: CaptureKey,
        /// Validated whole image with root and stable object evidence.
        snapshot: Box<ObservedSnapshot>,
    },
    /// Publish only after the coordinator validates the live capture lease.
    Activate {
        /// Immutable capture boundary and generation.
        key: CaptureKey,
        /// Published checkpoint identity.
        checkpoint: CheckpointId,
    },
    /// Fence a failed/expired candidate; retain its reservation.
    Unavailable {
        /// Immutable capture boundary and generation.
        key: CaptureKey,
        /// Typed failure retained for clients.
        reason: CheckpointError,
    },
    /// Persist a frozen preview, before any restore is authorized.
    Plan {
        /// Exact frozen effective preview.
        plan: Box<Plan>,
        /// Coordinator observation time.
        at: DateTime<Utc>,
    },
    /// Consume the exact preview and commit every Pending path intent atomically.
    Admit {
        /// Exact frozen effective preview.
        plan: Box<Plan>,
        /// Idempotency identity for the restore.
        restore: RestoreId,
        /// Coordinator observation time.
        at: DateTime<Utc>,
    },
    /// Commit every prepared original/written object before project mutation.
    Staged {
        /// Exact owned durable operation snapshot.
        operation: Box<Operation>,
        /// Complete canonical per-path object evidence.
        paths: vessel_proto::checkpoint::StagedPaths,
    },
    /// Compare-and-advance a path after its real filesystem postcondition.
    Advance {
        /// Exact owned durable operation snapshot.
        operation: Box<Operation>,
        /// Approved path whose postcondition was checked.
        path: CheckpointPath,
        /// Expected durable path phase.
        from: Phase,
        /// Verified next durable path phase.
        to: Phase,
    },
    /// Commit a terminal result consistent with all path intents.
    Outcome {
        /// Exact owned durable operation snapshot.
        operation: Box<Operation>,
        /// Verified whole-scope terminal outcome.
        result: RestoreOutcome,
    },
    /// Resolve recovery only after the whole original image and identity is verified.
    Recovered {
        /// Exact owned durable operation snapshot.
        operation: Box<Operation>,
        /// Whole pre-restore image and stable object evidence after recovery.
        observed: ObservedSnapshot,
        /// Typed failure retained for clients.
        reason: CheckpointError,
    },
    /// Reserve a fixed private worker envelope before allocating its files.
    Allocate {
        /// Immutable capture boundary and generation.
        key: CaptureKey,
        /// Non-reusable private storage identity.
        allocation: super::allocation::Allocation,
        /// Full authenticated human principal.
        owner: vessel_proto::checkpoint::CheckpointOwner,
        /// Coordinator observation time.
        at: DateTime<Utc>,
    },
    /// Actual worker finished; attach its retained bytes to published durable state.
    AllocationAttached {
        /// Immutable capture boundary and generation.
        key: CaptureKey,
        /// Non-reusable private storage identity.
        allocation: super::allocation::Allocation,
        /// Full authenticated human principal.
        owner: vessel_proto::checkpoint::CheckpointOwner,
    },
    /// Actual unpublished private allocation removed; retain a non-reusable id charge.
    AllocationCleaned {
        /// Immutable capture boundary and generation.
        key: CaptureKey,
        /// Non-reusable private storage identity.
        allocation: super::allocation::Allocation,
        /// Full authenticated human principal.
        owner: vessel_proto::checkpoint::CheckpointOwner,
    },
    /// Startup/lease invalidation never promotes a persisted candidate.
    InvalidateCandidates,
    /// Release a garbage charge only after its verified physical removal.
    GarbageCleaned {
        /// Typed garbage identity physically removed.
        object: super::GarbageObject,
    },
}

/// The writer's savepoint encloses an admission AND every Pending path intent.
pub(crate) fn apply(
    connection: &Connection,
    write: CheckpointWrite,
) -> Result<(), crate::StoreError> {
    let result = match write {
        CheckpointWrite::RefusedBoundary { key, reason, at } => {
            publication::refused(connection, key, reason, at)
        }
        CheckpointWrite::Collect { at } => super::rotation::collect(connection, at),
        CheckpointWrite::EndedMetadata { operation, at } => {
            return super::metadata::append_ended(connection, &operation, at);
        }
        CheckpointWrite::ReserveCapture { key, at } => {
            publication::boundary_persisted(connection, key)?;
            journal::candidate(connection, key, super::quota::CAPTURE_BYTES, at)
        }
        CheckpointWrite::CaptureSettled { key } => super::quota::capture_settled(connection, key),
        #[cfg(test)]
        CheckpointWrite::Reserve { key, bytes, at } => {
            journal::candidate(connection, key, bytes, at)?;
            super::quota::model_without_worker(connection, key)
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
        CheckpointWrite::Plan { plan, at } => {
            super::allocation::preview_reserved(connection, &plan)?;
            journal::put_plan(connection, &plan, at)
        }
        CheckpointWrite::Admit { plan, restore, at } => {
            let new = match journal::status(connection, plan.key.session, restore, plan.owner) {
                Ok(_existing) => false,
                Err(Error::Refused(CheckpointError::NotFound)) => true,
                Err(error) => return Err(error.into()),
            };
            let operation = journal::admit(connection, &plan, restore, at)?;
            if new {
                super::allocation::reserve(
                    connection,
                    plan.key,
                    super::Allocation::Restore { restore },
                    plan.owner,
                    at,
                )?;
                paths::prepare(connection, &operation)?;
            }
            Ok(())
        }
        CheckpointWrite::Staged { operation, paths } => {
            super::staging::commit(connection, &operation, &paths)
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
