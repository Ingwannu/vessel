//! Retained receipt admission (ADR 0084), owned by the one store writer.
//! Question claims stay staged until the actor/provider bridge is connected.

mod fingerprint;
pub(crate) mod observation;
#[cfg(test)]
mod observation_tests;
pub(crate) mod question;
mod receipt;
#[cfg(test)]
mod tests;

use rusqlite::{Connection, OptionalExtension, params};
use vessel_proto::SessionId;
use vessel_proto::timeline::admission::{
    AdmissionCapacity, AdmissionKind, AdmissionResource, AdmissionScope, MAX_NODE_ADMISSION_BYTES,
    MAX_NODE_ADMISSIONS, MAX_SESSION_ADMISSION_BYTES, MAX_SESSION_ADMISSIONS,
};

use crate::{ByteCount, StoreError, codec};
use vessel_proto::timeline::admission::AdmissionBytes;

use fingerprint::ReceiptFingerprint;
pub use question::{
    AnswerAdmission as QuestionAnswerAdmission, AnswerNotSent as QuestionAnswerNotSent,
    AnswerTarget as QuestionAnswerTarget, AnswerToken as QuestionAnswerToken, QuestionAdmission,
};
pub use receipt::{ReceiptAdmission, ReceiptToken};
use receipt::{admit_receipt, finish_receipt};

pub(crate) fn lookup_input(
    connection: &Connection,
    request: &vessel_proto::timeline::SubmitPrompt,
    creator: &vessel_proto::Creator,
) -> Result<Option<vessel_proto::timeline::SubmitPromptReply>, StoreError> {
    receipt::lookup_receipt(
        connection,
        request.session,
        request.message.clone(),
        ReceiptFingerprint::for_prompt(request, creator)?,
    )
}

/// All package tables will join the single integration-repointed migration.
pub(crate) const SQL: &str = "
-- Existing snapshots remain legacy-unscoped. Never let a later turn overwrite
-- the details of a pending approval belonging to an earlier turn.
ALTER TABLE inbox_tools RENAME TO inbox_tools_legacy;
CREATE TABLE inbox_tools (
    session TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    id TEXT NOT NULL,
    scope TEXT NOT NULL,
    seq INTEGER NOT NULL,
    tick INTEGER NOT NULL,
    tool TEXT NOT NULL,
    detail TEXT NOT NULL,
    PRIMARY KEY(session,id,scope)
) STRICT, WITHOUT ROWID;
INSERT INTO inbox_tools SELECT session,id,'',seq,tick,tool,detail FROM inbox_tools_legacy;
DROP TABLE inbox_tools_legacy;
CREATE TABLE timeline_admissions (
    session TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK(kind IN ('receipt', 'question')),
    identity TEXT NOT NULL,
    fingerprint BLOB NOT NULL,
    reserved_bytes INTEGER NOT NULL CHECK(reserved_bytes > 0),
    state TEXT NOT NULL,
    PRIMARY KEY(session, kind, identity)
) STRICT, WITHOUT ROWID;
CREATE UNIQUE INDEX timeline_question_request ON timeline_admissions
    (session,json_extract(state,'$.question.id')) WHERE kind='question';
CREATE TABLE timeline_admission_meta (
    singleton INTEGER NOT NULL PRIMARY KEY CHECK(singleton=1),
    records INTEGER NOT NULL CHECK(records>=0),
    reserved_bytes INTEGER NOT NULL CHECK(reserved_bytes>=0)
) STRICT;
INSERT INTO timeline_admission_meta VALUES (1,0,0);
CREATE TABLE timeline_admission_usage (
    session TEXT NOT NULL PRIMARY KEY REFERENCES sessions(id) ON DELETE CASCADE,
    records INTEGER NOT NULL CHECK(records>=0),
    reserved_bytes INTEGER NOT NULL CHECK(reserved_bytes>=0)
) STRICT, WITHOUT ROWID;
CREATE TRIGGER timeline_admission_insert AFTER INSERT ON timeline_admissions BEGIN
    UPDATE timeline_admission_meta SET records=records+1,
      reserved_bytes=reserved_bytes+new.reserved_bytes WHERE singleton=1;
    INSERT INTO timeline_admission_usage VALUES (new.session,1,new.reserved_bytes)
      ON CONFLICT(session) DO UPDATE SET records=records+1,
      reserved_bytes=reserved_bytes+new.reserved_bytes;
END;
CREATE TRIGGER timeline_admission_delete AFTER DELETE ON timeline_admissions BEGIN
    UPDATE timeline_admission_meta SET records=records-1,
      reserved_bytes=reserved_bytes-old.reserved_bytes WHERE singleton=1;
    UPDATE timeline_admission_usage SET records=records-1,
      reserved_bytes=reserved_bytes-old.reserved_bytes WHERE session=old.session;
END;
";

/// Counters are maintained transactionally; startup verifies them against retained rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Usage {
    records: u64,
    bytes: u64,
}

fn usage(connection: &Connection, session: Option<SessionId>) -> Result<Usage, StoreError> {
    let found: Option<(i64, i64)> = match session {
        Some(session) => connection
            .query_row(
                "SELECT records,reserved_bytes FROM timeline_admission_usage WHERE session=?1",
                [session.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?,
        None => Some(connection.query_row(
            "SELECT records,reserved_bytes FROM timeline_admission_meta WHERE singleton=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?),
    };
    let (records, bytes) = found.unwrap_or_default();
    Ok(Usage {
        records: u64::try_from(records).map_err(|_negative| StoreError::AdmissionIntegrity)?,
        bytes: u64::try_from(bytes).map_err(|_negative| StoreError::AdmissionIntegrity)?,
    })
}

fn capacity(
    used: Usage,
    kind: AdmissionKind,
    scope: AdmissionScope,
) -> Result<(), AdmissionCapacity> {
    let (max_records, max_bytes) = match scope {
        AdmissionScope::Session => (MAX_SESSION_ADMISSIONS, MAX_SESSION_ADMISSION_BYTES),
        AdmissionScope::Node => (MAX_NODE_ADMISSIONS, MAX_NODE_ADMISSION_BYTES),
    };
    let error = |resource| AdmissionCapacity { scope, resource };
    if used.records >= max_records {
        return Err(error(AdmissionResource::Records));
    }
    if used
        .bytes
        .checked_add(kind.reserved_bytes().get())
        .is_none_or(|bytes| bytes > max_bytes)
    {
        return Err(error(AdmissionResource::Bytes));
    }
    Ok(())
}

fn reserve(
    connection: &Connection,
    session: SessionId,
    kind: AdmissionKind,
) -> Result<Result<(), AdmissionCapacity>, StoreError> {
    // Only the one writer inside a transaction may perform lookup/reservation/insert.
    if connection.is_autocommit() {
        return Err(StoreError::AdmissionIntegrity);
    }
    if let Err(capacity) = capacity(
        usage(connection, Some(session))?,
        kind,
        AdmissionScope::Session,
    ) {
        return Ok(Err(capacity));
    }
    Ok(capacity(
        usage(connection, None)?,
        kind,
        AdmissionScope::Node,
    ))
}

/// Verify reserved charges at startup; corrupted accounting never permits fresh work.
pub(crate) fn verify(connection: &Connection) -> Result<(), StoreError> {
    let bad: i64 = connection.query_row(
        "SELECT count(*) FROM timeline_admissions WHERE
         (kind='receipt' AND (reserved_bytes<>?1 OR length(CAST(state AS BLOB))+length(CAST(identity AS BLOB))+length(fingerprint)>?3)) OR
         (kind='question' AND (reserved_bytes<>?2 OR length(CAST(state AS BLOB))+length(CAST(identity AS BLOB))+length(fingerprint)>?4))",
        params![sql_bytes(AdmissionKind::Receipt.reserved_bytes())?, sql_bytes(AdmissionKind::Question.reserved_bytes())?,
            sql_bytes(AdmissionBytes::new(vessel_proto::timeline::admission::MAX_RECEIPT_STATE_BYTES))?,
            sql_bytes(AdmissionBytes::new(vessel_proto::timeline::admission::MAX_QUESTION_STATE_BYTES))?],
        |row| row.get(0),
    )?;
    if bad != 0 {
        return Err(StoreError::AdmissionIntegrity);
    }
    let actual: (i64, i64) = connection.query_row(
        "SELECT count(*),coalesce(sum(reserved_bytes),0) FROM timeline_admissions",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let kept = usage(connection, None)?;
    if u64::try_from(actual.0).ok() != Some(kept.records)
        || u64::try_from(actual.1).ok() != Some(kept.bytes)
    {
        return Err(StoreError::AdmissionIntegrity);
    }
    let bad_sessions:i64=connection.query_row(
        "SELECT count(*) FROM timeline_admission_usage AS u WHERE
         u.records<>(SELECT count(*) FROM timeline_admissions AS a WHERE a.session=u.session)
         OR u.reserved_bytes<>(SELECT coalesce(sum(a.reserved_bytes),0) FROM timeline_admissions AS a WHERE a.session=u.session)",[],|row|row.get(0),
    )?;
    let missing: i64 = connection.query_row(
        "SELECT count(*) FROM timeline_admissions AS a WHERE NOT EXISTS
         (SELECT 1 FROM timeline_admission_usage AS u WHERE u.session=a.session)",
        [],
        |row| row.get(0),
    )?;
    if bad_sessions != 0 || missing != 0 {
        return Err(StoreError::AdmissionIntegrity);
    }
    Ok(())
}

fn sql_bytes(bytes: AdmissionBytes) -> Result<i64, StoreError> {
    codec::bytes_to_sql(ByteCount::new(bytes.get()))
}

/// The caller validates provider-specific content and authorization before admission.
/// The store additionally refuses the common syntactically empty input before hashing.
pub(crate) fn admit_input(
    connection: &Connection,
    request: &vessel_proto::timeline::SubmitPrompt,
    creator: &vessel_proto::Creator,
) -> Result<ReceiptAdmission, StoreError> {
    if request.content.is_empty() {
        return Err(StoreError::EmptyIdentifiedPrompt);
    }
    let fingerprint = ReceiptFingerprint::for_prompt(request, creator)?;
    admit_receipt(
        connection,
        request.session,
        request.message.clone(),
        fingerprint,
    )
}

pub(crate) fn accept_input(
    connection: &Connection,
    token: ReceiptToken,
    accepted: vessel_proto::timeline::SubmissionAcceptedKind,
) -> Result<vessel_proto::timeline::PromptReceipt, StoreError> {
    finish_receipt(connection, token, accepted)
}

/// Settle only a fresh reservation with an actor-confirmed no-effect refusal.
pub(crate) fn refuse_input(
    connection: &Connection,
    token: ReceiptToken,
    reason: vessel_proto::timeline::SubmissionRefusal,
) -> Result<vessel_proto::timeline::SubmitPromptReply, StoreError> {
    receipt::refuse_receipt(connection, token, reason)
}
