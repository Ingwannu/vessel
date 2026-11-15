//! Diagnostic source used by the bounded node collector (ADR 0074).
//! Source owners omit private paths, arbitrary text and credentials.
use std::num::TryFromIntError;
use vessel_proto::diagnostics::{
    DiagnosticSchemaVersion, InvalidDiagnostic, MAX_DIAGNOSTIC_SCHEMA,
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum DiagnosticSchemaError {
    #[error("diagnostic schema must be unsigned and at most {max}", max = MAX_DIAGNOSTIC_SCHEMA)]
    Representation(#[source] TryFromIntError),
    #[error("diagnostic supported schema violates its bound: {0}")]
    Bound(#[from] InvalidDiagnostic),
}

/// The actual migration-list length, never an independently assigned number.
///
/// # Errors
/// A supported version outside the diagnostic schema representation.
pub(crate) fn version() -> Result<DiagnosticSchemaVersion, DiagnosticSchemaError> {
    let version =
        u32::try_from(super::MIGRATIONS.len()).map_err(DiagnosticSchemaError::Representation)?;
    Ok(DiagnosticSchemaVersion::try_from(version)?)
}
