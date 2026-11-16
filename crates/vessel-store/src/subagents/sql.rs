//! Canonical current-parent validation inside the statement selecting an effect.
//!
//! [Decision Log]
//! - Purpose: detached children are independent without rewriting `Creator` audit.
//! - Assets: typed `Creator`/`DetachReceipt` decoding and `Parentage::of` already own
//!   the receipt/provenance rule. A SQL non-null/JSON-field shortcut would copy
//!   that rule and could admit a corrupt or foreign receipt.
//! - Choice: a deterministic, innocuous scalar over exactly the row's two stored
//!   columns. Candidate filtering stays before LIMIT; source and lease checks
//!   share their statement's snapshot, with no second read or post-filter scan.
//! - Invariant: every connection runs `schema::configure`. Missing registration is
//!   a SQL error, never a fallback to audit `Creator` or an affirmative role.
//! - Cost: existing `rusqlite` functions feature, no new crate/schema/wire, and
//!   bounded typed decoding of the small provenance fields for candidate rows.
use crate::StoreError;
use rusqlite::{Connection, Error, functions::FunctionFlags, types::ValueRef};
use serde::de::DeserializeOwned;
use vessel_proto::{
    Creator,
    subagents::{DetachReceipt, Parentage},
};

// Known Creator/DetachReceipt encodings contain fixed-size UUIDs and one time.
// Refuse corrupt oversized fields before allocating or deserializing a tree.
const MAX_PARENTAGE_JSON_BYTES: usize = 1024;

#[derive(Debug, thiserror::Error)]
#[error("stored session parentage is invalid")]
struct InvalidStoredParentage;

pub(crate) fn configure(connection: &Connection) -> Result<(), StoreError> {
    connection.create_scalar_function(
        "vessel_has_current_parent",
        2,
        FunctionFlags::SQLITE_UTF8
            | FunctionFlags::SQLITE_DETERMINISTIC
            | FunctionFlags::SQLITE_INNOCUOUS,
        |context| {
            let creator = optional::<Creator>(context.get_raw(0))?.unwrap_or_default();
            let receipt = optional::<DetachReceipt>(context.get_raw(1))?;
            Parentage::of(&creator, receipt)
                .map(|parentage| parentage.parent().is_some())
                .map_err(|_invalid| invalid())
        },
    )?;
    Ok(())
}
fn optional<T: DeserializeOwned>(value: ValueRef<'_>) -> Result<Option<T>, Error> {
    match value {
        ValueRef::Null => Ok(None),
        ValueRef::Text(text) if text.len() <= MAX_PARENTAGE_JSON_BYTES => {
            serde_json::from_slice(text)
                .map(Some)
                .map_err(|_invalid| invalid())
        }
        ValueRef::Integer(_) | ValueRef::Real(_) | ValueRef::Text(_) | ValueRef::Blob(_) => {
            Err(invalid())
        }
    }
}
fn invalid() -> Error {
    Error::UserFunctionError(Box::new(InvalidStoredParentage))
}

#[cfg(test)]
mod tests;
