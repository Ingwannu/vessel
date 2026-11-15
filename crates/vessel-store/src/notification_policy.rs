//! Session-origin eligibility shared by push candidates and local/send
//! admission. Keep internal/delegated work in the inbox without notifying it.
/// Every caller joins sessions as `s`; the predicate is static trusted SQL.
/// Filtering before LIMIT preserves progress past pages of internal work.
pub(crate) const SESSION: &str = "s.archived_at IS NULL AND (s.created_by IS NULL OR json_extract(s.created_by,'$.kind') IN ('owner','device')) AND json_extract(s.spec,'$.side_question_of') IS NULL";
