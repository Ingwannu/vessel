//! Use node-owned inbox rollups without redefining `SessionInfo` log marks.
//! Unsupported peers retain their explicitly labelled legacy read status.
use crate::shell::Shell;
use vessel_proto::{SessionId, inbox_attention::SessionAttention};
impl Shell {
    pub(crate) fn inbox_session_attention(
        &self,
        machine: &str,
        session: SessionId,
    ) -> Option<SessionAttention> {
        let context = self
            .fleet
            .machines
            .iter()
            .find(|item| item.name == machine)?
            .connection_context()?;
        self.areas
            .inbox
            .store
            .nodes()
            .iter()
            .find(|item| *item.source.context() == context)?
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
                attention.explicit_unread || attention.unread.get() != 0
            })
    }
    pub(crate) fn inbox_bucket(
        &self,
        card: &vessel_app_view::Card,
    ) -> vessel_app_view::inbox::buckets::Bucket {
        vessel_app_view::inbox::buckets::Bucket::of(
            card,
            self.inbox_session_attention(&card.machine, card.session),
        )
    }
    pub(crate) fn inbox_status_groups(
        &self,
        view: vessel_app_view::prefs::SidebarView,
    ) -> Vec<vessel_app_view::inbox::buckets::Group<'_>> {
        vessel_app_view::inbox::buckets::groups(
            &self.fleet,
            view,
            vessel_app_view::sidebar::SIDEBAR_STATUS_SESSIONS,
            |card| self.inbox_session_attention(&card.machine, card.session),
        )
    }
    pub(crate) fn inbox_sidebar_order(&self) -> Vec<&vessel_app_view::Card> {
        let view = self.prefs.settings.sidebar;
        match view.group {
            vessel_app_view::prefs::Grouping::Machine => self.fleet.sidebar_order(view),
            vessel_app_view::prefs::Grouping::Status => self
                .inbox_status_groups(view)
                .into_iter()
                .flat_map(|group| group.sessions)
                .collect(),
        }
    }
}
