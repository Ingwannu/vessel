//! Passive retargeting is atomic and cannot bypass lifecycle or modified-work guards.
use super::{AgentRole, CloseEffect, Layout, LayoutError, OpenIntent, PaneHost, TabId, Target};
/// Owner-supplied replacement after its draft/create or preview workflow succeeds.
#[derive(Debug, Clone)]
pub struct RetargetRequest {
    /// Stable original presentation identity.
    pub tab: TabId,
    /// New typed resource identity.
    pub target: Target,
    /// Background completion never steals selection.
    pub intent: OpenIntent,
    /// Hydrated role, or Unknown until the lifecycle owner resolves it.
    pub role: AgentRole,
}
impl Layout {
    /// Replace a clean passive preview/draft; equal resources reveal their existing tab.
    ///
    /// # Errors
    /// Refuses missing tabs, dirty work, lifecycle-owned targets or unsupported hosts.
    pub fn retarget(&mut self, request: RetargetRequest) -> Result<TabId, LayoutError> {
        if matches!(request.target, Target::Plugin { .. }) {
            return Err(LayoutError::UnsupportedHost);
        }
        let mut next = self.clone();
        let current = next.tab(request.tab).ok_or(LayoutError::MissingTab)?;
        let plan = next.request_close(request.tab)?;
        if current.modified() != super::Modified::Clean {
            return Err(LayoutError::Modified);
        }
        if plan.effect() != CloseEffect::Local {
            return Err(LayoutError::UnsupportedHost);
        }
        let destination = next.containing(request.tab);
        let host = match destination {
            Some(super::PaneId::EXPLORER) => PaneHost::Explorer,
            Some(_) => PaneHost::Main,
            None => PaneHost::Sheet,
        };
        if !request.target.supports(host) {
            return Err(LayoutError::UnsupportedHost);
        }
        let existing = next
            .tabs()
            .find(|tab| tab.id() != request.tab && tab.target().same_resource(&request.target))
            .map(super::Tab::id);
        let id = if let Some(existing) = existing {
            next.close_confirmed(&plan, super::CloseApproval::Local)?;
            if request.intent != OpenIntent::Background {
                next.select(existing)?;
            }
            existing
        } else {
            let tab = next
                .tabs
                .get_mut(&request.tab)
                .ok_or(LayoutError::MissingTab)?;
            tab.target = request.target;
            tab.role = super::close::AgentCloseRole::from(request.role);
            request.tab
        };
        *self = next;
        Ok(id)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{LayoutMode, Modified, Placement};
    #[test]
    fn replacement_preserves_tab_id_and_stale_close_cannot_remove_new_content() {
        let mut layout = Layout::default();
        let tab = layout
            .open(
                Target::NewTab,
                OpenIntent::New,
                Placement::Focused,
                AgentRole::Unknown,
                LayoutMode::SplitPanes,
            )
            .unwrap();
        let stale = layout.request_close(tab).unwrap();
        let target = Target::Draft(crate::layout::DraftId::random());
        assert_eq!(
            layout
                .retarget(RetargetRequest {
                    tab,
                    target: target.clone(),
                    intent: OpenIntent::Reveal,
                    role: AgentRole::Unknown
                })
                .unwrap(),
            tab
        );
        assert_eq!(
            layout.close_confirmed(&stale, crate::layout::CloseApproval::Local),
            Err(LayoutError::InvalidState)
        );
        layout.set_modified(tab, Modified::Dirty).unwrap();
        assert_eq!(
            layout.retarget(RetargetRequest {
                tab,
                target: Target::NewTab,
                intent: OpenIntent::Reveal,
                role: AgentRole::Unknown
            }),
            Err(LayoutError::Modified)
        );
        assert_eq!(layout.tab(tab).unwrap().target(), &target);
    }
}
