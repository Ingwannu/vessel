//! What a session row and the open picker read from a host's followed
//! labels (`follow.rs`): the row's registry workspace and its labels in
//! their host colours, and the Labels item that opens the picker there.
//! Kept apart from the reads that keep those current.

use vessel_app_kit::ui::Context;
use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::labels::{EditAccess, Label};

use super::PickerPlace;
use crate::shell::Shell;

/// A session row's workspace labels: its host's admission, its registry
/// workspace, its labels in order with their host colours, and whether
/// the host takes changes now.
pub(crate) struct RowLabels<'a> {
    pub(crate) connection: &'a ConnectionContext,
    pub(crate) workspace: vessel_proto::WorkspaceId,
    pub(crate) labels: Vec<Label>,
    pub(crate) writable: bool,
}

impl Shell {
    /// The picker for `session`'s workspace, its list at `place` (a row
    /// menu's Labels page beside it or on a phone); false when the session
    /// has no workspace on a followed host.
    pub(crate) fn labels_row_picker(
        &mut self,
        machine: &str,
        session: vessel_proto::SessionId,
        project: Option<vessel_proto::ProjectId>,
        place: PickerPlace,
        window: &mut vessel_app_kit::ui::Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        let Some(row) = self.areas.labels.row(machine, session, project) else {
            return false;
        };
        let key = vessel_app_view::layout::LayoutKey::Workspace {
            node: row.connection.node(),
            workspace: row.workspace,
        };
        let Ok(target) =
            vessel_app_view::labels::WorkspaceTarget::new(row.connection.clone(), &key)
        else {
            return false;
        };
        let assigned = row.labels.iter().map(|label| label.name.clone()).collect();
        let access = if row.writable {
            EditAccess::Editable
        } else {
            EditAccess::ReadOnly
        };
        self.labels_picker_open_in(target, assigned, access, place, window, cx)
    }

    /// The open picker's assigned labels and access, from its host's
    /// followed registry.
    pub(super) fn labels_picker_refresh(&mut self, cx: &mut Context<'_, Self>) {
        let Some((target, assigned, access)) = self.areas.labels.picker_view() else {
            return;
        };
        self.labels_picker_sync(&target, assigned, access, cx);
    }
}

impl super::Labels {
    /// The open picker's target with its registry labels and access now.
    fn picker_view(
        &self,
    ) -> Option<(
        vessel_app_view::labels::WorkspaceTarget,
        Vec<vessel_app_view::labels::Name>,
        EditAccess,
    )> {
        let target = self.picker_target()?;
        let follow = self
            .follows
            .iter()
            .find(|follow| &follow.connection == target.connection())?;
        let assigned = follow.registry.labels(target.workspace()).to_vec();
        let writable = self
            .hosts
            .iter()
            .find(|host| host.key == follow.key)
            .is_some_and(|host| host.state.writable().is_ok());
        let access = if writable {
            EditAccess::Editable
        } else {
            EditAccess::ReadOnly
        };
        Some((target, assigned, access))
    }

    /// A session row's workspace and labels, on `machine`'s host: none
    /// while the host is not followed or the session has no workspace.
    /// `project` is the card's
    /// [`member_project`](vessel_app_view::board::Card::member_project): a
    /// new session's derived membership advances no registry revision, so
    /// the row takes it from there rather than wait for the next snapshot.
    pub(crate) fn row(
        &self,
        machine: &str,
        session: vessel_proto::SessionId,
        project: Option<vessel_proto::ProjectId>,
    ) -> Option<RowLabels<'_>> {
        let host = self
            .hosts
            .iter()
            .find(|host| host.route.as_str() == machine)?;
        let connection = host.connection.as_ref()?;
        let follow = self
            .follows
            .iter()
            .find(|follow| &follow.connection == connection)?;
        let workspace = follow.registry.workspace(session, project)?;
        let labels = follow
            .registry
            .labels(workspace)
            .iter()
            .map(|name| {
                // A name the catalog does not hold yet (its read is behind
                // the registry's) keeps the create colour until it arrives.
                host.catalog.get(name).cloned().unwrap_or_else(|| Label {
                    name: name.clone(),
                    color: vessel_app_view::labels::Color::default(),
                })
            })
            .collect();
        Some(RowLabels {
            connection,
            workspace,
            labels,
            writable: host.state.writable().is_ok(),
        })
    }
}
