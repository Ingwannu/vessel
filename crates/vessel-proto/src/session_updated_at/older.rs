//! Hide optional mutation time after every existing session metadata projection.

use crate::{ProtocolVersion, SessionInfo};

impl SessionInfo {
    /// Apply this optional feature after all existing metadata projections.
    pub(crate) fn for_protocol_with_mutation_time(
        &self,
        protocol: ProtocolVersion,
        assigned: Option<ProtocolVersion>,
    ) -> Option<Self> {
        let projected =
            self.for_protocol_with_parentage(protocol, crate::subagents::SUBAGENT_RELATIONS);
        let source = projected.as_ref().unwrap_or(self);
        if source.updated_at.is_none() || assigned.is_some_and(|since| since <= protocol) {
            return projected;
        }
        let mut info = projected.unwrap_or_else(|| self.clone());
        info.updated_at = None;
        Some(info)
    }
}

#[cfg(test)]
mod tests;
