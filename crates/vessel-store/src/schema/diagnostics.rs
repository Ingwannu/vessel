//! Typed supported-schema projection, staged under cfg(test) until wiring.
//! Shared admission/method metadata wiring removes staging. Migration SQL
//! and the canonical migration list remain owned by the parent unchanged.
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
