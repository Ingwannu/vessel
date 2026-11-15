//! Receipt admission and completion, both inside the single writer transaction.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use vessel_proto::timeline::admission::AdmissionKind;
use vessel_proto::timeline::{ClientMessageId, PromptReceipt, SubmitPromptReply};
use vessel_proto::{Delivered, SessionId};

use super::{ReceiptFingerprint, reserve};
use crate::{Column, StoreError, codec};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum ReceiptState {
    Pending,
    Accepted { delivered: Delivered },
}

/// A fresh admission is the only result authorizing the caller to dispatch once.
#[derive(Debug)]
pub(super) enum ReceiptAdmission {
    New(ReceiptToken),
    Existing(SubmitPromptReply),
}

/// Unforgeable outside this owner: completion can only settle a fresh reservation.
#[derive(Debug)]
pub(super) struct ReceiptToken {
    session: SessionId,
    message: ClientMessageId,
    fingerprint: ReceiptFingerprint,
}

pub(super) fn admit_receipt(
    connection: &Connection,
    session: SessionId,
    message: ClientMessageId,
    fingerprint: ReceiptFingerprint,
) -> Result<ReceiptAdmission, StoreError> {
    if connection.is_autocommit() {
        return Err(StoreError::AdmissionIntegrity);
    }
    let exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sessions WHERE id=?1)",
        [session.to_string()],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(StoreError::UnknownSession(session));
    }
    let known: Option<(Vec<u8>, String)> = connection
        .query_row(
            "SELECT fingerprint,state FROM timeline_admissions
         WHERE session=?1 AND kind='receipt' AND identity=?2",
            params![session.to_string(), message.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    if let Some((known_fingerprint, state)) = known {
        let reply = replay(session, message, fingerprint, known_fingerprint, &state)?;
        return Ok(ReceiptAdmission::Existing(reply));
    }
    if let Err(capacity) = reserve(connection, session, AdmissionKind::Receipt)? {
        return Ok(ReceiptAdmission::Existing(SubmitPromptReply::Capacity {
            message,
            capacity,
        }));
    }
    let pending = codec::encode(&ReceiptState::Pending)?;
    check_envelope(&message, &fingerprint, &pending)?;
    let _inserted = connection.execute(
        "INSERT INTO timeline_admissions(session,kind,identity,fingerprint,reserved_bytes,state)
         VALUES (?1,'receipt',?2,?3,?4,?5)",
        params![
            session.to_string(),
            message.as_str(),
            fingerprint.bytes().as_slice(),
            AdmissionKind::Receipt.reserved_bytes(),
            codec::encode(&ReceiptState::Pending)?
        ],
    )?;
    Ok(ReceiptAdmission::New(ReceiptToken {
        session,
        message,
        fingerprint,
    }))
}

fn replay(
    session: SessionId,
    message: ClientMessageId,
    fingerprint: ReceiptFingerprint,
    known: Vec<u8>,
    state: &str,
) -> Result<SubmitPromptReply, StoreError> {
    if ReceiptFingerprint::from_sql(known)? != fingerprint {
        return Ok(SubmitPromptReply::KeyConflict { message });
    }
    Ok(match codec::decode(session, Column::Admission, state)? {
        ReceiptState::Pending => SubmitPromptReply::OutcomeUnknown { message },
        ReceiptState::Accepted { delivered } => SubmitPromptReply::Accepted {
            receipt: PromptReceipt { message, delivered },
        },
    })
}

pub(super) fn finish_receipt(
    connection: &Connection,
    token: ReceiptToken,
    delivered: Delivered,
) -> Result<PromptReceipt, StoreError> {
    if connection.is_autocommit() {
        return Err(StoreError::AdmissionIntegrity);
    }
    let state = codec::encode(&ReceiptState::Accepted { delivered })?;
    check_envelope(&token.message, &token.fingerprint, &state)?;
    let pending = codec::encode(&ReceiptState::Pending)?;
    let changed = connection.execute(
        "UPDATE timeline_admissions SET state=?1 WHERE session=?2 AND kind='receipt'
         AND identity=?3 AND fingerprint=?4 AND state=?5",
        params![
            state,
            token.session.to_string(),
            token.message.as_str(),
            token.fingerprint.bytes().as_slice(),
            pending
        ],
    )?;
    if changed != 1 {
        return Err(StoreError::AdmissionIntegrity);
    }
    Ok(PromptReceipt {
        message: token.message,
        delivered,
    })
}

fn check_envelope(
    message: &ClientMessageId,
    fingerprint: &ReceiptFingerprint,
    state: &str,
) -> Result<(), StoreError> {
    let size = message
        .as_str()
        .len()
        .checked_add(fingerprint.bytes().len())
        .and_then(|size| size.checked_add(state.len()))
        .ok_or(StoreError::AdmissionIntegrity)?;
    let encoded = u64::try_from(size).map_err(|_too_large| StoreError::AdmissionIntegrity)?;
    if !AdmissionKind::Receipt.fits(encoded) {
        return Err(StoreError::AdmissionIntegrity);
    }
    Ok(())
}
