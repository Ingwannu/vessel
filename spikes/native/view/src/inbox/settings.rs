//! GPUI submits notifications without a delivery or permission result. The
//! UI can show a request, never infer that the system accepted or displayed it.
/// Result the current backend can actually establish.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TestNotice {
    /// No test has been requested in this window.
    #[default]
    Idle,
    /// A request was handed to the backend; display remains unconfirmed.
    Requested,
}
