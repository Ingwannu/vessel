//! Palette commands retain their original presentation instead of following focus.
use super::{Layout, LayoutKey, Pane, PaneId, TabId, Target};

/// A captured UI selection, never permission to issue a node request.
///
/// Private fields prevent a caller from replacing only the target after a menu
/// was opened. Identity-scoped effects still need their resident/connection lease.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutActionContext {
    workspace: LayoutKey,
    pane: PaneId,
    selected: Option<(TabId, Target)>,
    sheet: Option<(TabId, Target)>,
}
impl LayoutActionContext {
    /// Original workspace, including the explicit older-session bridge.
    #[must_use]
    pub const fn workspace(&self) -> &LayoutKey {
        &self.workspace
    }
}
impl Layout {
    /// Capture before opening a palette; execution must validate it again.
    #[must_use]
    pub fn action_context(&self, workspace: &LayoutKey) -> LayoutActionContext {
        let selection = |id| self.tab(id).map(|tab| (id, tab.target().clone()));
        LayoutActionContext {
            workspace: workspace.clone(),
            pane: self.focused,
            selected: self
                .root
                .pane(self.focused)
                .and_then(Pane::focused)
                .and_then(selection),
            sheet: self.sheets.last().and_then(|sheet| selection(sheet.tab)),
        }
    }
    /// Reordering background tabs may continue; changing the selected resource,
    /// pane, workspace or top compact sheet invalidates the captured command.
    #[must_use]
    pub fn accepts_action_context(
        &self,
        workspace: &LayoutKey,
        context: &LayoutActionContext,
    ) -> bool {
        workspace == context.workspace() && self.action_context(workspace) == *context
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{
        AgentRole, DraftId, LayoutMode, OpenIntent, Placement, Position, RetargetRequest,
    };
    use vessel_proto::{NodeId, WorkspaceId};

    fn workspace() -> LayoutKey {
        LayoutKey::Workspace {
            node: NodeId::random(),
            workspace: WorkspaceId::random(),
        }
    }
    fn open(layout: &mut Layout, target: Target, intent: OpenIntent) -> TabId {
        layout
            .open(
                target,
                intent,
                Placement::Focused,
                AgentRole::Unknown,
                LayoutMode::SplitPanes,
            )
            .unwrap()
    }
    #[test]
    fn captured_workspace_and_resource_cannot_follow_a_new_selection() {
        let mut layout = Layout::default();
        let key = workspace();
        let tab = open(&mut layout, Target::NewTab, OpenIntent::New);
        let context = layout.action_context(&key);
        assert!(layout.accepts_action_context(&key, &context));
        assert!(!layout.accepts_action_context(&workspace(), &context));
        let _background = open(&mut layout, Target::NewTab, OpenIntent::Background);
        assert!(layout.accepts_action_context(&key, &context));
        let _retargeted = layout
            .retarget(RetargetRequest {
                tab,
                target: Target::Draft(DraftId::random()),
                intent: OpenIntent::Reveal,
                role: AgentRole::Unknown,
            })
            .unwrap();
        assert!(!layout.accepts_action_context(&key, &context));
    }
    #[test]
    fn focus_and_compact_sheet_changes_refuse_a_pending_palette_action() {
        let key = workspace();
        let mut layout = Layout::default();
        let _tab = open(&mut layout, Target::NewTab, OpenIntent::New);
        let context = layout.action_context(&key);
        let _side = layout
            .split(PaneId::MAIN, Position::Right, LayoutMode::SplitPanes)
            .unwrap();
        assert!(!layout.accepts_action_context(&key, &context));
        let context = layout.action_context(&key);
        let _sheet = layout
            .open(
                Target::File {
                    root: vessel_proto::workspace::WorkspaceRoot::Project {
                        project: vessel_proto::ProjectId::random(),
                    },
                    path: "src/main.rs".parse().unwrap(),
                },
                OpenIntent::New,
                Placement::Focused,
                AgentRole::Unknown,
                LayoutMode::Compact,
            )
            .unwrap();
        assert!(!layout.accepts_action_context(&key, &context));
    }
}
