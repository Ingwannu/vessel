//! Compact reveal policy preserves the same resource and protects nested dirty work.
use super::{AgentRole, Layout, LayoutError, Modified, Sheet, SheetPromotion, TabId, Target};
pub(super) const fn policy(target: &Target, role: AgentRole) -> Option<SheetPromotion> {
    match (target, role) {
        (Target::Agent(_), AgentRole::Child) => Some(SheetPromotion::Allowed),
        (Target::File { .. } | Target::ProviderSubagent { .. }, _) => {
            Some(SheetPromotion::Unavailable)
        }
        (
            Target::NewTab
            | Target::Draft(_)
            | Target::Agent(_)
            | Target::Terminal(_)
            | Target::Browser(_)
            | Target::Changes
            | Target::Files
            | Target::Scripts
            | Target::PullRequest
            | Target::WorkingDiff
            | Target::Plugin { .. }
            | Target::Setup(_)
            | Target::CommitDiff(_),
            _,
        ) => None,
    }
}
impl Layout {
    pub(super) fn reveal_sheet(
        &mut self,
        id: TabId,
        promotion: SheetPromotion,
    ) -> Result<(), LayoutError> {
        if let Some(index) = self.sheets.iter().position(|sheet| sheet.tab == id) {
            let later: Vec<_> = self
                .sheets
                .iter()
                .skip(index.saturating_add(1))
                .map(|sheet| sheet.tab)
                .collect();
            if later.iter().any(|id| {
                self.tabs
                    .get(id)
                    .is_some_and(|tab| tab.modified == Modified::Dirty)
            }) {
                return Err(LayoutError::Modified);
            }
            for id in later {
                self.detach(id);
                let _closed = self.tabs.remove(&id);
            }
        } else {
            let parent = self.sheets.last().map(|sheet| sheet.tab);
            self.detach(id);
            self.sheets.push(Sheet {
                tab: id,
                parent,
                promotion,
            });
        }
        Ok(())
    }
}
