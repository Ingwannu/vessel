//! Use node-owned inbox rollups without redefining SessionInfo log marks.
//! Unsupported peers retain their explicitly labelled legacy read status.
use crate::shell::Shell;
use vessel_proto::{SessionId, inbox_attention::SessionAttention};
impl Shell {
    pub(crate) fn inbox_session_attention(
        &self,
        machine: &str,
        session: SessionId,
    ) -> Option<SessionAttention> {
        let node = self
            .fleet
            .machines
            .iter()
            .find(|item| item.name == machine)?
            .node?;
        self.areas
            .inbox
            .store
            .nodes()
            .iter()
            .find(|item| item.source.node == node && item.source.name.as_str() == machine)?
            .snapshot
            .as_ref()?
            .attention
            .as_ref()?
            .iter()
            .find(|attention| attention.session == session)
            .copied()
    }
    pub(crate) fn inbox_unread(&self, card: &vessel_app_view::Card) -> bool {
        self.inbox_session_attention(&card.machine, card.session)
            .map_or(card.detail.marks.unread, |attention| {
                attention.unread.get() != 0
            })
    }
}
