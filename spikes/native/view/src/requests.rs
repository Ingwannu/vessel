//! Request identity shared by resource owners, without response admission state.
use vessel_proto::{PermissionId, QuestionId};

/// Keep permission and question identities distinct across owner callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Request {
    /// A provider-offered permission choice.
    Permission(PermissionId),
    /// A typed question form.
    Question(QuestionId),
}
