//! Durable nonblocking question admission and answer claims, without provider I/O.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use vessel_proto::timeline::QuestionDeliveryState;
use vessel_proto::timeline::admission::{AdmissionCapacity, AdmissionKind};
use vessel_proto::{
    EventSeq, Question, QuestionAnswer, QuestionId, QuestionOutcome, SessionId, ToolCallId,
};

use super::reserve;
use crate::{Column, StoreError, codec};

/// Foreground selection is captured before admission and checked again by the actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "target", rename_all = "snake_case")]
pub enum AnswerTarget {
    /// Answer only the foreground turn captured before claim admission.
    Turn {
        /// Canonical turn start in the durable session log.
        started: EventSeq,
    },
    /// No foreground turn: the answer may start one new follow-up.
    Idle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum QuestionState {
    Pending,
    Answering {
        answer: QuestionAnswer,
        target: AnswerTarget,
    },
    OutcomeUnknown {
        answer: QuestionAnswer,
        target: AnswerTarget,
    },
    Settled {
        outcome: QuestionOutcome,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Record {
    question: Question,
    source: ToolCallId,
    state: QuestionState,
}

/// A stable source identity deduplicates a completed provider item after restart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestionAdmission {
    /// Newly retained actionable question.
    New(Question),
    /// Invalid generic companion metadata never becomes actionable.
    InvalidForm(vessel_proto::conversation::InvalidQuestionPresentation),
    /// Retained source replay, with current delivery state.
    Existing(Question),
    /// Retained source/request was already resolved.
    Settled,
    /// A stable source was reused for a different form.
    Conflict,
    /// No reservation was made because a named finite budget is full.
    Capacity(AdmissionCapacity),
    /// The complete form or answer exceeds its reserved envelope.
    TooLarge,
}

/// A response cannot be duplicated while an admitted claim might have reached a provider.
#[derive(Debug)]
pub enum AnswerAdmission {
    /// Only this opaque token may authorize one answer dispatch.
    New(AnswerToken),
    /// An admitted answer must not dispatch again.
    AlreadyClaimed(QuestionDeliveryState),
    /// Retained source/request was already resolved.
    Settled,
    /// No retained durable request has this identity.
    Missing,
    /// The typed answer does not fit the stored form.
    Invalid(vessel_proto::conversation::InvalidAnswer),
    /// The complete form or answer exceeds its reserved envelope.
    TooLarge,
}

/// Only failures known to precede dispatch may return a claim to Pending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerNotSent {
    /// Foreground turn changed before any answer effect.
    TurnChanged,
    /// The provider cannot take this answer in the captured state.
    Unsupported,
    /// Validation rejected the answer before sending anything.
    RejectedBeforeSend,
}

/// Only an admitted exact answer can complete or roll back its own claim.
#[derive(Debug)]
pub struct AnswerToken {
    session: SessionId,
    source: ToolCallId,
    record: Record,
}

impl AnswerToken {
    /// The authoritative validated form associated with this one claim.
    #[must_use]
    pub const fn question(&self) -> &Question {
        &self.record.question
    }
    /// The admitted answer, retained independently of a driver's responder lifetime.
    ///
    /// # Errors
    /// The token no longer contains an admitted in-flight answer.
    #[must_use]
    pub fn answer(&self) -> Result<&QuestionAnswer, StoreError> {
        if let QuestionState::Answering { answer, .. } = &self.record.state {
            Ok(answer)
        } else {
            Err(StoreError::AdmissionIntegrity)
        }
    }
}

/// Read the authoritative durable requests, excluding settled replay tombstones.
pub(crate) fn pending(
    connection: &Connection,
    session: SessionId,
) -> Result<Vec<Question>, StoreError> {
    let mut statement = connection.prepare("SELECT state FROM timeline_admissions WHERE session=?1 AND kind='question' ORDER BY identity")?;
    let rows = statement.query_map([session.to_string()], |row| row.get::<_, String>(0))?;
    let mut questions = Vec::new();
    for state in rows {
        let record: Record = codec::decode(session, Column::Admission, &state?)?;
        if let Some(question) = displayed(&record) {
            questions.push(question);
        }
    }
    Ok(questions)
}

/// An abandoned admitted answer is uncertain immediately, not only after restart.
pub(crate) fn abandon_answer(
    connection: &Connection,
    mut token: AnswerToken,
) -> Result<(), StoreError> {
    require_claim(connection, &token)?;
    let QuestionState::Answering { answer, target } = token.record.state else {
        return Err(StoreError::AdmissionIntegrity);
    };
    token.record.state = QuestionState::OutcomeUnknown { answer, target };
    let encoded = codec::encode(&token.record)?;
    update(connection, token.session, &token.record, &encoded)
}

pub(crate) fn admit_question(
    connection: &Connection,
    session: SessionId,
    source: ToolCallId,
    mut question: Question,
) -> Result<QuestionAdmission, StoreError> {
    require_transaction(connection)?;
    if let Err(error) = question.validate_presentation() {
        return Ok(QuestionAdmission::InvalidForm(error));
    }
    let exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sessions WHERE id=?1)",
        [session.to_string()],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(StoreError::UnknownSession(session));
    }
    // This owner admits provider-classified async questions only. Delivery state
    // is a store projection, never a state a provider can assert.
    question.lifetime = vessel_proto::timeline::QuestionLifetime::Session;
    question.delivery_state = QuestionDeliveryState::Pending;
    let known = load(connection, session, &source)?;
    if let Some(known) = known {
        // A driver's freshly minted node id is not the provider's stable source identity.
        if known.question.message != question.message
            || known.question.fields != question.fields
            || known.question.tool_call != question.tool_call
        {
            return Ok(QuestionAdmission::Conflict);
        }
        return Ok(match displayed(&known) {
            Some(question) => QuestionAdmission::Existing(question),
            None => QuestionAdmission::Settled,
        });
    }
    let record = Record {
        question,
        source,
        state: QuestionState::Pending,
    };
    let encoded = codec::encode(&record)?;
    if !fits(&record.source, &encoded)? {
        return Ok(QuestionAdmission::TooLarge);
    }
    if let Err(capacity) = reserve(connection, session, AdmissionKind::Question)? {
        return Ok(QuestionAdmission::Capacity(capacity));
    }
    let _inserted = connection.execute(
        "INSERT INTO timeline_admissions(session,kind,identity,fingerprint,reserved_bytes,state)
         VALUES (?1,'question',?2,X'',?3,?4)",
        params![
            session.to_string(),
            record.source.as_str(),
            super::sql_bytes(AdmissionKind::Question.reserved_bytes())?,
            encoded
        ],
    )?;
    Ok(QuestionAdmission::New(record.question))
}

pub(crate) fn begin_answer(
    connection: &Connection,
    session: SessionId,
    id: QuestionId,
    answer: QuestionAnswer,
    target: AnswerTarget,
) -> Result<AnswerAdmission, StoreError> {
    require_transaction(connection)?;
    let state: Option<String> = connection
        .query_row(
            "SELECT state FROM timeline_admissions WHERE session=?1 AND kind='question'
         AND json_extract(state,'$.question.id')=?2",
            params![session.to_string(), id.to_string()],
            |row| row.get(0),
        )
        .optional()?;
    let Some(state) = state else {
        return Ok(AnswerAdmission::Missing);
    };
    let mut record: Record = codec::decode(session, Column::Admission, &state)?;
    match record.state {
        QuestionState::Answering { .. } => {
            return Ok(AnswerAdmission::AlreadyClaimed(
                QuestionDeliveryState::Answering,
            ));
        }
        QuestionState::OutcomeUnknown { .. } => {
            return Ok(AnswerAdmission::AlreadyClaimed(
                QuestionDeliveryState::OutcomeUnknown,
            ));
        }
        QuestionState::Settled { .. } => return Ok(AnswerAdmission::Settled),
        QuestionState::Pending => {}
    }
    if let Err(error) = record.question.check(&answer) {
        return Ok(AnswerAdmission::Invalid(error));
    }
    record.state = QuestionState::Answering { answer, target };
    let encoded = codec::encode(&record)?;
    if !fits(&record.source, &encoded)? {
        return Ok(AnswerAdmission::TooLarge);
    }
    update(connection, session, &record, &encoded)?;
    Ok(AnswerAdmission::New(AnswerToken {
        session,
        source: record.source.clone(),
        record,
    }))
}

/// Success means the actor observed provider acceptance; resolution does not precede it.
pub(crate) fn finish_answer(
    connection: &Connection,
    mut token: AnswerToken,
) -> Result<QuestionOutcome, StoreError> {
    require_claim(connection, &token)?;
    let outcome = match &token.record.state {
        QuestionState::Answering {
            answer: QuestionAnswer::Accept { values },
            ..
        } => QuestionOutcome::Answered {
            values: values.clone(),
        },
        QuestionState::Answering {
            answer: QuestionAnswer::Decline,
            ..
        } => QuestionOutcome::Declined,
        QuestionState::Pending
        | QuestionState::OutcomeUnknown { .. }
        | QuestionState::Settled { .. } => return Err(StoreError::AdmissionIntegrity),
    };
    token.record.state = QuestionState::Settled {
        outcome: outcome.clone(),
    };
    let encoded = codec::encode(&token.record)?;
    update(connection, token.session, &token.record, &encoded)?;
    Ok(outcome)
}

/// Only a typed known-not-sent result may release an answer claim for a retry.
pub(crate) fn reject_before_effect(
    connection: &Connection,
    mut token: AnswerToken,
    reason: AnswerNotSent,
) -> Result<(), StoreError> {
    require_claim(connection, &token)?;
    match reason {
        AnswerNotSent::TurnChanged
        | AnswerNotSent::Unsupported
        | AnswerNotSent::RejectedBeforeSend => {}
    }
    token.record.state = QuestionState::Pending;
    let encoded = codec::encode(&token.record)?;
    update(connection, token.session, &token.record, &encoded)
}

/// A lost acknowledgement keeps the answer non-actionable across restart.
pub(crate) fn recover_claims(connection: &Connection) -> Result<(), StoreError> {
    require_transaction(connection)?;
    let mut statement = connection
        .prepare("SELECT session,state FROM timeline_admissions WHERE kind='question'")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (session, state) = row?;
        let session = codec::session_id_from_sql(&session)?;
        let mut record: Record = codec::decode(session, Column::Admission, &state)?;
        if let QuestionState::Answering { answer, target } = record.state {
            record.state = QuestionState::OutcomeUnknown { answer, target };
            let encoded = codec::encode(&record)?;
            update(connection, session, &record, &encoded)?;
        }
    }
    Ok(())
}

fn displayed(record: &Record) -> Option<Question> {
    let state = match record.state {
        QuestionState::Pending => QuestionDeliveryState::Pending,
        QuestionState::Answering { .. } => QuestionDeliveryState::Answering,
        QuestionState::OutcomeUnknown { .. } => QuestionDeliveryState::OutcomeUnknown,
        QuestionState::Settled { .. } => return None,
    };
    let mut question = record.question.clone();
    question.delivery_state = state;
    Some(question)
}

fn require_claim(connection: &Connection, token: &AnswerToken) -> Result<(), StoreError> {
    require_transaction(connection)?;
    let kept = load(connection, token.session, &token.source)?;
    if kept.as_ref() != Some(&token.record) {
        return Err(StoreError::AdmissionIntegrity);
    }
    Ok(())
}

fn require_transaction(connection: &Connection) -> Result<(), StoreError> {
    if connection.is_autocommit() {
        return Err(StoreError::AdmissionIntegrity);
    }
    Ok(())
}

fn load(
    connection: &Connection,
    session: SessionId,
    source: &ToolCallId,
) -> Result<Option<Record>, StoreError> {
    let state:Option<String>=connection.query_row(
        "SELECT state FROM timeline_admissions WHERE session=?1 AND kind='question' AND identity=?2",
        params![session.to_string(),source.as_str()],|row|row.get(0),
    ).optional()?;
    state
        .map(|state| codec::decode(session, Column::Admission, &state))
        .transpose()
}

fn update(
    connection: &Connection,
    session: SessionId,
    record: &Record,
    encoded: &str,
) -> Result<(), StoreError> {
    if !fits(&record.source, encoded)? {
        return Err(StoreError::AdmissionIntegrity);
    }
    let changed=connection.execute("UPDATE timeline_admissions SET state=?1 WHERE session=?2 AND kind='question' AND identity=?3",
        params![encoded,session.to_string(),record.source.as_str()])?;
    if changed != 1 {
        return Err(StoreError::AdmissionIntegrity);
    }
    Ok(())
}

fn fits(source: &ToolCallId, encoded: &str) -> Result<bool, StoreError> {
    let size = source
        .as_str()
        .len()
        .checked_add(encoded.len())
        .ok_or(StoreError::AdmissionIntegrity)?;
    Ok(
        AdmissionKind::Question.fits(vessel_proto::timeline::admission::AdmissionBytes::new(
            u64::try_from(size).map_err(|_too_large| StoreError::AdmissionIntegrity)?,
        )),
    )
}

#[cfg(test)]
mod tests;
