//! Pending report methods fail closed until their source registry is executable.
//! No fabricated empty/successful response may hide missing source support.
use super::{Connection, NodeService};
use vessel_proto::{
    ErrorCode, WireError,
    usage_reports::{InspectUsageReport, ListUsageReports, UsageReportLookup, UsageReports},
};
impl NodeService {
    pub(crate) async fn list_usage_reports(
        &self,
        _connection: &Connection,
        _request: ListUsageReports,
    ) -> Result<UsageReports, WireError> {
        Err(unavailable())
    }
    pub(crate) async fn inspect_usage_report(
        &self,
        _connection: &Connection,
        _request: InspectUsageReport,
    ) -> Result<UsageReportLookup, WireError> {
        Err(unavailable())
    }
}
fn unavailable() -> WireError {
    WireError::new(
        ErrorCode::Unsupported,
        "account report sources are not available in this build",
    )
}
