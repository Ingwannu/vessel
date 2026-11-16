//! A disconnected picker retains input, never an acknowledgement or authority.
use vessel_app_view::orchestration::{Load, Page, Target};
use vessel_proto::skills::SkillSelection;

/// One volatile draft, restored only after this same host answers a new read.
/// Tickets, visits, confirmations and installed-state snapshots are not carried.
pub(super) struct Draft {
    target: Target,
    selection: SkillSelection,
}
impl Draft {
    pub(super) fn capture(page: &Page) -> Option<Self> {
        Some(Self {
            target: page.scope().target.clone(),
            selection: page.picker.as_ref()?.draft.clone(),
        })
    }
    pub(super) fn matches(&self, target: &Target) -> bool {
        self.target.machine() == target.machine() && self.target.node() == target.node()
    }
    pub(super) fn restore(self, page: &mut Page) -> bool {
        if !self.matches(&page.scope().target) || !matches!(&page.skills, Load::Ready(_)) {
            return false;
        }
        // The new page owns its own visit and fresh snapshot. Only checkbox
        // input survives; save and removal confirmation must start again.
        page.choose();
        let Some(picker) = &mut page.picker else {
            return false;
        };
        picker.draft = self.selection;
        true
    }
}
