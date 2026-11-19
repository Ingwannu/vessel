//! Session-origin eligibility shared by push candidates and local/send
//! admission. Current parentage, not immutable Creator audit or a workspace-
//! relative root role, controls delegation. The configured canonical scalar
//! validates full detach receipts inside each caller's existing SQL snapshot.
/// Every caller joins sessions as `s`; the predicate is static trusted SQL.
/// Filtering before LIMIT preserves progress past pages of internal work.
pub(crate) const SESSION: &str = "s.archived_at IS NULL AND NOT vessel_has_current_parent(s.created_by,s.detach_receipt) AND json_extract(s.spec,'$.side_question_of') IS NULL";
