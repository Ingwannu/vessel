//! Actual actor targets advance independently of immutable receipt acceptance.
//! The fresh owner's private token and one writer prevent public read identities
//! or stale callbacks from replacing an already settled target.

use rusqlite::{Connection, params};
use vessel_proto::timeline::{PromptDisposition, PromptReceiptBinding, SubmissionAcceptedKind};
use vessel_proto::{SessionId, timeline::ClientMessageId};

use super::ReceiptFingerprint;
use super::receipt::{ReceiptBindingToken, ReceiptState, check_envelope, read_receipt};
use crate::{Column, StoreError, codec};

pub(crate) fn record(
    connection: &Connection,
    token: ReceiptBindingToken,
    target: PromptReceiptBinding,
) -> Result<(), StoreError> {
    if connection.is_autocommit() {
        return Err(StoreError::AdmissionIntegrity);
    }
    let known = read_receipt(connection, token.session, &token.message)?
        .ok_or(StoreError::AdmissionIntegrity)?;
    if ReceiptFingerprint::from_sql(known.fingerprint)? != token.fingerprint {
        return Err(StoreError::AdmissionIntegrity);
    }
    let ReceiptState::Accepted { accepted } =
        codec::decode(token.session, Column::Admission, &known.state)?
    else {
        return Err(StoreError::AdmissionIntegrity);
    };
    let current = decode(token.session, accepted, known.binding.as_deref())?;
    validate(accepted, target)?;
    if current == target {
        return Ok(());
    }
    if !matches!(
        (current, target),
        (
            PromptReceiptBinding::Unobserved,
            PromptReceiptBinding::Turn { .. }
        ) | (
            PromptReceiptBinding::Queued { .. },
            PromptReceiptBinding::Turn { .. } | PromptReceiptBinding::Withdrawn { .. }
        )
    ) {
        return Err(StoreError::AdmissionIntegrity);
    }
    let encoded = codec::encode(&target)?;
    check_envelope(
        &token.message,
        &token.fingerprint,
        &known.state,
        Some(&encoded),
    )?;
    let changed = connection.execute(
        "UPDATE timeline_admissions SET binding=?1 WHERE session=?2 AND kind='receipt'
         AND identity=?3 AND fingerprint=?4 AND state=?5 AND binding IS ?6",
        params![
            encoded,
            token.session.to_string(),
            token.message.as_str(),
            token.fingerprint.bytes().as_slice(),
            known.state,
            known.binding
        ],
    )?;
    if changed != 1 {
        return Err(StoreError::AdmissionIntegrity);
    }
    Ok(())
}

pub(super) fn initial(accepted: SubmissionAcceptedKind) -> PromptReceiptBinding {
    match accepted {
        SubmissionAcceptedKind::Prompt {
            disposition: PromptDisposition::Queued { entry },
        } => PromptReceiptBinding::Queued { entry },
        SubmissionAcceptedKind::Command { echo, .. } => PromptReceiptBinding::Command { echo },
        SubmissionAcceptedKind::Prompt {
            disposition:
                PromptDisposition::Started
                | PromptDisposition::Steered
                | PromptDisposition::InterruptedAndStarted { .. },
        } => PromptReceiptBinding::Unobserved,
    }
}

pub(super) fn decode(
    session: SessionId,
    accepted: SubmissionAcceptedKind,
    encoded: Option<&str>,
) -> Result<PromptReceiptBinding, StoreError> {
    let target = encoded.map_or_else(
        || Ok(initial(accepted)),
        |text| codec::decode(session, Column::Admission, text),
    )?;
    validate(accepted, target)?;
    Ok(target)
}

fn validate(
    accepted: SubmissionAcceptedKind,
    target: PromptReceiptBinding,
) -> Result<(), StoreError> {
    let compatible = match target {
        PromptReceiptBinding::Command { echo } => matches!(accepted,
            SubmissionAcceptedKind::Command { echo: actual, .. } if actual == echo),
        PromptReceiptBinding::Queued { entry } | PromptReceiptBinding::Withdrawn { entry } => {
            matches!(accepted,
            SubmissionAcceptedKind::Prompt { disposition: PromptDisposition::Queued { entry: actual } } if actual == entry)
        }
        PromptReceiptBinding::Unobserved => matches!(
            accepted,
            SubmissionAcceptedKind::Prompt {
                disposition: PromptDisposition::Started
                    | PromptDisposition::Steered
                    | PromptDisposition::InterruptedAndStarted { .. }
            }
        ),
        PromptReceiptBinding::Turn { started } => match accepted {
            SubmissionAcceptedKind::Command { .. } => false,
            SubmissionAcceptedKind::Prompt { disposition } => match disposition {
                PromptDisposition::InterruptedAndStarted { interrupted } => interrupted != started,
                PromptDisposition::Started
                | PromptDisposition::Steered
                | PromptDisposition::Queued { .. } => true,
            },
        },
    };
    if !compatible {
        return Err(StoreError::AdmissionIntegrity);
    }
    Ok(())
}

pub(crate) fn verify(connection: &Connection) -> Result<(), StoreError> {
    let mut statement = connection.prepare("SELECT session,identity,fingerprint,state,binding FROM timeline_admissions WHERE binding IS NOT NULL")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let session = codec::session_id_from_sql(&row.get::<_, String>(0)?)?;
        let message: ClientMessageId = row
            .get::<_, String>(1)?
            .parse()
            .map_err(|_invalid| StoreError::AdmissionIntegrity)?;
        let fingerprint = ReceiptFingerprint::from_sql(row.get(2)?)?;
        let state: String = row.get(3)?;
        let encoded: String = row.get(4)?;
        let ReceiptState::Accepted { accepted } =
            codec::decode(session, Column::Admission, &state)?
        else {
            return Err(StoreError::AdmissionIntegrity);
        };
        let _validated = decode(session, accepted, Some(&encoded))?;
        check_envelope(&message, &fingerprint, &state, Some(&encoded))?;
    }
    Ok(())
}
