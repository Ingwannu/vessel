//! Writer-serialized delegation metadata, independent of session lifecycle.
//!
//! The immutable creator is the audit truth. A receipt is committed once in
//! the same savepoint as the first/repeat decision; lost callers cannot cause
//! a later retry to mint another timestamp. No process or event-log write occurs.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use vessel_proto::{
    SessionId,
    subagents::{DetachDisposition, DetachReceipt, DetachedSession, Parentage},
};

use crate::{codec::encode, error::StoreError, reader::select_session};

pub(crate) fn detach(
    connection: &Connection,
    session: SessionId,
    at: DateTime<Utc>,
) -> Result<DetachedSession, StoreError> {
    let mut info =
        select_session(connection, session)?.ok_or(StoreError::UnknownSession(session))?;
    let parentage = info
        .parentage()
        .map_err(|_invalid| StoreError::InvalidParentage(session))?;
    let (receipt, disposition) = match parentage {
        Parentage::Root => return Err(StoreError::NotChild(session)),
        Parentage::Detached(receipt) => (receipt, DetachDisposition::AlreadyDetached),
        Parentage::Attached(parent) => {
            let receipt = DetachReceipt::new(parent, at);
            let _changed = connection.execute(
                "UPDATE sessions SET detach_receipt=?2 WHERE id=?1",
                params![session.to_string(), encode(&receipt)?],
            )?;
            (receipt, DetachDisposition::Detached)
        }
    };
    info.detach_receipt = Some(receipt);
    Ok(DetachedSession {
        session: Box::new(info),
        receipt,
        disposition,
    })
}
