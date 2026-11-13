//! Aggregate reads use the store's existing bounded blocking pool.
use crate::{Store, StoreError};
use chrono::{DateTime, Utc};
use vessel_proto::usage::{QueryUsage, UsageReport};
impl Store {
    /// Reported usage as of `now`; no provider or credential I/O.
    ///
    /// # Errors
    /// The database read failed or a recorded report is corrupt.
    pub async fn usage(
        &self,
        request: QueryUsage,
        now: DateTime<Utc>,
    ) -> Result<UsageReport, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| crate::usage::query(connection, &request, now))
        })
        .await
    }
}
