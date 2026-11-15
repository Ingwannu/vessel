//! Diagnostic source used by the bounded node collector (ADR 0074).
//! Source owners omit private paths, arbitrary text and credentials.
use vessel_harness::{AddedAgent, AddedAgents};
use vessel_proto::{
    HarnessKind,
    diagnostics::{
        CustomAgentSlot, DiagnosticAgentId, DiagnosticUnavailable, InvalidDiagnostic,
        MAX_DIAGNOSTIC_AGENTS, MIN_CUSTOM_AGENT_SLOT,
    },
};

pub(super) enum Entry<'a> {
    Builtin(HarnessKind),
    Added {
        slot: CustomAgentSlot,
        source: &'a AddedAgent,
    },
}
impl Entry<'_> {
    pub(super) fn id(&self) -> DiagnosticAgentId {
        match self {
            Self::Builtin(harness) => DiagnosticAgentId::Builtin { harness: *harness },
            Self::Added { slot, .. } => DiagnosticAgentId::Custom { slot: *slot },
        }
    }
}
pub(super) struct Inventory<'a> {
    pub(super) entries: Vec<Entry<'a>>,
}
impl std::fmt::Debug for Inventory<'_> {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.debug_struct("Inventory")
            .field("entries", &self.entries.len())
            .finish()
    }
}
#[derive(Debug, thiserror::Error)]
pub(super) enum SelectionError {
    #[error("diagnostic inventory exceeds {limit} agents", limit = MAX_DIAGNOSTIC_AGENTS)]
    TooMany,
    #[error("diagnostic report-local agent identity violates its bound: {0}")]
    Slot(#[from] InvalidDiagnostic),
}
impl SelectionError {
    pub(super) fn unavailable(&self) -> DiagnosticUnavailable {
        match self {
            Self::TooMany => DiagnosticUnavailable::TooManyAgents,
            Self::Slot(_) => DiagnosticUnavailable::ReadFailed,
        }
    }
}
impl<'a> Inventory<'a> {
    pub(super) fn select(configured: &'a AddedAgents) -> Result<Self, SelectionError> {
        let total = HarnessKind::ALL
            .len()
            .checked_add(configured.all().count())
            .filter(|count| *count <= MAX_DIAGNOSTIC_AGENTS)
            .ok_or(SelectionError::TooMany)?;
        let mut entries = Vec::with_capacity(total);
        entries.extend(HarnessKind::ALL.into_iter().map(Entry::Builtin));
        for (index, agent) in configured.all().enumerate() {
            let ordinal = index
                .checked_add(MIN_CUSTOM_AGENT_SLOT)
                .ok_or(SelectionError::TooMany)?;
            let slot = CustomAgentSlot::try_from(ordinal)?;
            entries.push(Entry::Added {
                slot,
                source: agent.as_ref(),
            });
        }
        Ok(Self { entries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;
    const PRIVATE_WORDS: &str = "private-diagnostic-source-canary";
    fn configured(count: usize) -> AddedAgents {
        let mut text = String::new();
        for index in 0..count {
            writeln!(text, "[agents.private-agent-{index}]\nprogram = \"{PRIVATE_WORDS}\"\nlabel = \"{PRIVATE_WORDS}\"\nargs = [\"{PRIVATE_WORDS}\"]\n").unwrap();
        }
        vessel_harness::parse_agents(&text, None).unwrap()
    }
    #[test]
    fn exact_cap_is_complete_borrowed_and_exposes_only_report_local_ids() {
        let custom = MAX_DIAGNOSTIC_AGENTS
            .checked_sub(HarnessKind::ALL.len())
            .unwrap();
        let config = configured(custom);
        let inventory = Inventory::select(&config).unwrap();
        assert_eq!(inventory.entries.len(), MAX_DIAGNOSTIC_AGENTS);
        let ids = inventory.entries.iter().map(Entry::id).collect::<Vec<_>>();
        assert_eq!(
            ids.iter().collect::<std::collections::HashSet<_>>().len(),
            ids.len()
        );
        assert!(!serde_json::to_string(&ids).unwrap().contains(PRIVATE_WORDS));
        assert!(!format!("{inventory:?}").contains(PRIVATE_WORDS));
        let borrowed = inventory
            .entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Builtin(_) => None,
                Entry::Added { source, .. } => Some(*source),
            })
            .collect::<Vec<_>>();
        assert_eq!(borrowed.len(), custom);
        for (selected, original) in borrowed.into_iter().zip(config.all()) {
            assert!(std::ptr::eq(selected, original.as_ref()));
        }
    }
    #[test]
    fn excess_inventory_is_unavailable_instead_of_a_complete_truncated_list() {
        let excess = MAX_DIAGNOSTIC_AGENTS
            .checked_sub(HarnessKind::ALL.len())
            .unwrap()
            .checked_add(1)
            .unwrap();
        let config = configured(excess);
        let error = Inventory::select(&config).unwrap_err();
        assert_eq!(error.unavailable(), DiagnosticUnavailable::TooManyAgents);
        assert!(
            error
                .to_string()
                .contains(&MAX_DIAGNOSTIC_AGENTS.to_string())
        );
        assert!(!format!("{error:?}").contains(PRIVATE_WORDS));
    }
}
