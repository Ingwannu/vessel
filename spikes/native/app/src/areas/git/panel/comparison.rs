//! Empty-state navigation uses the same normalized comparison and reader.
//! No mode is approximated on an older node; the link keeps Unsupported honest.
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_link::git_panel::Ask;
use vessel_app_view::workspace::git::panel::{
    Comparison, ComparisonMode, ComparisonTicket, PanelAddress, PanelTarget, ReadFailure,
    RepositoryKey,
};

pub(super) struct ComparisonChange {
    pub(super) address: PanelAddress,
    pub(super) target: PanelTarget,
    pub(super) instance: vessel_app_view::workspace::git::RequestKey,
    pub(super) context: vessel_app_view::connection::ConnectionContext,
    pub(super) repository: RepositoryKey,
    pub(super) comparison: Comparison,
    pub(super) summary: ComparisonTicket,
}

impl ComparisonChange {
    pub(super) fn is_current(
        &self,
        entry: &super::Entry,
        current: Option<&vessel_app_view::connection::ConnectionContext>,
    ) -> bool {
        entry.matches_admission(self.instance, &self.target, &self.context, current)
            && matches!(
                entry.target,
                PanelTarget::WorkingDiff | PanelTarget::Changes
            )
            && entry.reads.as_ref().is_some_and(|reads| {
                reads.repository() == self.repository
                    && reads.comparison() == &self.comparison
                    && reads.summary_ticket() == Some(&self.summary)
            })
    }
}

impl Shell {
    pub(super) fn git_panel_compare(
        &mut self,
        captured: &ComparisonChange,
        mode: ComparisonMode,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) {
        if self.areas.git.panels.registration.problem().is_some() {
            return;
        }
        let address = &captured.address;
        if !self.layout_tab_matches(
            &address.workspace,
            address.tab,
            &super::layout_target(&captured.target),
        ) || !self.layout_panel_visible(&address.workspace, address.tab, window)
        {
            return;
        }
        let current = self.git_panel_context(address);
        let git = &mut self.areas.git;
        let Some(entry) = git.panels.entries.get_mut(address) else {
            return;
        };
        if !captured.is_current(entry, current.as_ref()) {
            return;
        }
        let Some(reads) = &mut entry.reads else {
            return;
        };
        let Ok(key) = git.keys.issue() else {
            entry.problem = Some(ReadFailure::KeysExhausted);
            cx.notify();
            return;
        };
        let Some(context) = current else {
            return;
        };
        let comparison = Comparison::new(
            mode,
            captured.comparison.base().cloned(),
            captured.comparison.whitespace(),
        );
        entry.comparison = comparison.clone();
        reads.select(comparison);
        let ticket = reads.begin_summary(key);
        entry.files.reset(0);
        entry.path_places.clear();
        entry.documents.clear();
        entry.tips = vessel_app_kit::Tooltips::new();
        entry.problem = None;
        self.command(vessel_app_link::Command::GitPanel(Ask::Summary {
            address: address.clone(),
            context,
            ticket,
        }));
        cx.notify();
    }
}
