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
    CaptureKey, CheckpointError, CheckpointId, CheckpointPath, FileImage, RestoreId, RestoreOutcome,
};

#[derive(Debug)]
pub(crate) enum CheckpointWrite {
    Reserve {
        key: CaptureKey,
        bytes: crate::ByteCount,
        at: DateTime<Utc>,
    },
    Candidate {
        key: CaptureKey,
        image: FileImage,
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
    InvalidateCandidates,
}

/// The writer's savepoint encloses an admission AND every Pending path intent.
pub(crate) fn apply(connection: &Connection, write: CheckpointWrite) -> Result<(), Error> {
    match write {
        CheckpointWrite::Reserve { key, bytes, at } => {
            journal::candidate(connection, key, bytes, at)
        }
        CheckpointWrite::Candidate { key, image } => {
            journal::keep_candidate(connection, key, &image)
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
                Err(error) => return Err(error),
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
        CheckpointWrite::InvalidateCandidates => journal::invalidate_candidates(connection),
    }
}
