//! Receipt admission and completion, both inside the single writer transaction.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use vessel_proto::SessionId;
use vessel_proto::timeline::admission::AdmissionKind;
use vessel_proto::timeline::{
    ClientMessageId, PromptReceipt, SubmissionAcceptedKind, SubmissionRefusal, SubmitPromptReply,
};

use super::{ReceiptFingerprint, reserve};
use crate::{Column, StoreError, codec};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(super) enum ReceiptState {
    Pending,
    Accepted { accepted: SubmissionAcceptedKind },
    Refused { reason: SubmissionRefusal },
}

/// A fresh admission is the only result authorizing the caller to dispatch once.
#[derive(Debug)]
pub enum ReceiptAdmission {
    /// The Pending reservation committed before the caller may dispatch once.
    New(ReceiptToken),
    /// A replay, conflict, uncertainty or capacity refusal must not dispatch.
    Existing(SubmitPromptReply),
}

/// Unforgeable outside this owner: completion can only settle a fresh reservation.
#[derive(Debug)]
pub struct ReceiptToken {
    session: SessionId,
    message: ClientMessageId,
    fingerprint: ReceiptFingerprint,
}

/// Only a fresh receipt owner can retain this capability for actor-ticket settlement.
/// Public identities and read-only observation cannot construct one.
#[derive(Debug)]
pub struct ReceiptBindingToken {
    pub(super) session: SessionId,
    pub(super) message: ClientMessageId,
    pub(super) fingerprint: ReceiptFingerprint,
}

impl ReceiptToken {
    /// Retain before acceptance consumes the fresh reservation token. This grants
    /// no provider work; it can only settle this already accepted receipt's target.
    #[must_use]
    pub fn binding_token(&self) -> ReceiptBindingToken {
        ReceiptBindingToken {
            session: self.session,
            message: self.message.clone(),
            fingerprint: self.fingerprint,
        }
    }
}

pub(super) struct ReceiptRecord {
    pub(super) fingerprint: Vec<u8>,
    pub(super) state: String,
    pub(super) binding: Option<String>,
}

pub(super) fn read_receipt(
    connection: &Connection,
    session: SessionId,
    message: &ClientMessageId,
) -> Result<Option<ReceiptRecord>, StoreError> {
    Ok(connection
        .query_row(
            "SELECT fingerprint,state,binding FROM timeline_admissions
         WHERE session=?1 AND kind='receipt' AND identity=?2",
            params![session.to_string(), message.as_str()],
            |row| {
                Ok(ReceiptRecord {
                    fingerprint: row.get(0)?,
                    state: row.get(1)?,
                    binding: row.get(2)?,
                })
            },
        )
        .optional()?)
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
    if let Some(reply) = lookup_receipt(connection, session, message.clone(), fingerprint)? {
        return Ok(ReceiptAdmission::Existing(reply));
    }
    if let Err(capacity) = reserve(connection, session, AdmissionKind::Receipt)? {
        return Ok(ReceiptAdmission::Existing(SubmitPromptReply::Capacity {
            message,
            capacity,
        }));
    }
    let pending = codec::encode(&ReceiptState::Pending)?;
    check_envelope(&message, &fingerprint, &pending, None)?;
    let _inserted = connection.execute(
        "INSERT INTO timeline_admissions(session,kind,identity,fingerprint,reserved_bytes,state)
         VALUES (?1,'receipt',?2,?3,?4,?5)",
        params![
            session.to_string(),
            message.as_str(),
            fingerprint.bytes().as_slice(),
            super::sql_bytes(AdmissionKind::Receipt.reserved_bytes())?,
            pending
        ],
    )?;
    Ok(ReceiptAdmission::New(ReceiptToken {
        session,
        message,
        fingerprint,
    }))
}

pub(super) fn lookup_receipt(
    connection: &Connection,
    session: SessionId,
    message: ClientMessageId,
    fingerprint: ReceiptFingerprint,
) -> Result<Option<SubmitPromptReply>, StoreError> {
    if let Some(known) = read_receipt(connection, session, &message)? {
        let reply = replay(
            session,
            message,
            fingerprint,
            known.fingerprint,
            &known.state,
        )?;
        return Ok(Some(reply));
    }
    Ok(None)
}

pub(super) fn replay(
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
        ReceiptState::Accepted { accepted } => SubmitPromptReply::Accepted {
            receipt: PromptReceipt { message, accepted },
        },
        ReceiptState::Refused { reason } => SubmitPromptReply::Refused { message, reason },
    })
}

pub(super) fn finish_receipt(
    connection: &Connection,
    token: ReceiptToken,
    accepted: SubmissionAcceptedKind,
) -> Result<PromptReceipt, StoreError> {
    let message = settle_receipt(connection, token, &ReceiptState::Accepted { accepted })?;
    Ok(PromptReceipt { message, accepted })
}

pub(super) fn refuse_receipt(
    connection: &Connection,
    token: ReceiptToken,
    reason: SubmissionRefusal,
) -> Result<SubmitPromptReply, StoreError> {
    let message = settle_receipt(connection, token, &ReceiptState::Refused { reason })?;
    Ok(SubmitPromptReply::Refused { message, reason })
}

fn settle_receipt(
    connection: &Connection,
    token: ReceiptToken,
    state: &ReceiptState,
) -> Result<ClientMessageId, StoreError> {
    if connection.is_autocommit() {
        return Err(StoreError::AdmissionIntegrity);
    }
    let state = codec::encode(state)?;
    check_envelope(&token.message, &token.fingerprint, &state, None)?;
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
    Ok(token.message)
}

pub(super) fn check_envelope(
    message: &ClientMessageId,
    fingerprint: &ReceiptFingerprint,
    state: &str,
    binding: Option<&str>,
) -> Result<(), StoreError> {
    let size = message
        .as_str()
        .len()
        .checked_add(fingerprint.bytes().len())
        .and_then(|size| size.checked_add(state.len()))
        .and_then(|size| size.checked_add(binding.map_or(0, str::len)))
        .ok_or(StoreError::AdmissionIntegrity)?;
    let encoded = u64::try_from(size).map_err(|_too_large| StoreError::AdmissionIntegrity)?;
    if !AdmissionKind::Receipt.fits(vessel_proto::timeline::admission::AdmissionBytes::new(
        encoded,
    )) {
        return Err(StoreError::AdmissionIntegrity);
    }
    Ok(())
}
