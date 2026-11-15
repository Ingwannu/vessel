//! A detached write keeps its original intention and receipt after its UI retires.
//! This value has no replay method and grants no successor action authority.
use super::{Action, Target};
use crate::workspace::git::RequestKey;
use crate::workspace::problem::Problem;
use vessel_proto::scripts::ScriptRunOutcome;

/// Original sent payload retained independently of a retired screen's tickets.
#[derive(Debug)]
pub struct RetiredWrite {
    target: Target,
    key: RequestKey,
    action: Action,
    receipt: Option<Result<Option<ScriptRunOutcome>, Problem>>,
}
impl RetiredWrite {
    pub(super) fn new(target: Target, key: RequestKey, action: Action) -> Self {
        Self {
            target,
            key,
            action,
            receipt: None,
        }
    }
    /// Exact owner captured when the request was sent.
    #[must_use]
    pub const fn target(&self) -> &Target {
        &self.target
    }
    /// Exact original operation, never reconstructed from current selection.
    #[must_use]
    pub const fn action(&self) -> &Action {
        &self.action
    }
    /// An error receipt can still mean an unknown remote outcome.
    #[must_use]
    pub const fn receipt(&self) -> Option<&Result<Option<ScriptRunOutcome>, Problem>> {
        self.receipt.as_ref()
    }
    /// Retain only the original response, without applying it to a new screen.
    pub fn accept(
        &mut self,
        target: &Target,
        key: RequestKey,
        result: Result<Option<ScriptRunOutcome>, Problem>,
    ) -> bool {
        if &self.target != target || self.key != key || self.receipt.is_some() {
            return false;
        }
        self.receipt = Some(result);
        true
    }
}
