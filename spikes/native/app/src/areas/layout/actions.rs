//! Typed entry points shared by the tab rail, menu and sole shortcut registry.
use super::{mode, with_layout};
use crate::shell::Shell;
use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::layout::{
    AgentRole, CloseApproval, CloseEffect, ClosePlan, LayoutAction, LayoutError, Modified,
    OpenIntent, Pane, PaneId, Placement, Position, TabId, Target,
};

impl Shell {
    /// Panel owners open resources without owning layout or creating tab RPCs.
    pub(crate) fn layout_open_target(
        &mut self,
        workspace: &vessel_app_view::layout::LayoutKey,
        target: Target,
        intent: OpenIntent,
        placement: Placement,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Result<TabId, LayoutError> {
        let layout_mode = mode(window);
        self.update_layout(
            workspace,
            |layout| layout.open(target, intent, placement, AgentRole::Unknown, layout_mode),
            cx,
        )
    }
    /// Feature owners can apply only validated local operations, bound to their
    /// captured workspace. No panel receives the underlying book or raw fields.
    pub(crate) fn update_layout<T>(
        &mut self,
        workspace: &vessel_app_view::layout::LayoutKey,
        change: impl FnOnce(&mut vessel_app_view::layout::Layout) -> Result<T, LayoutError>,
        cx: &mut Context<'_, Self>,
    ) -> Result<T, LayoutError> {
        let before = super::registry::retirement::Before::layout(self, workspace);
        let result = super::with_layout_at(self, workspace, change);
        if result.is_ok() {
            self.layout_retired(before, super::registry::retirement::Source::Local, cx);
        }
        self.layout_result(result.as_ref().map(|_| ()).map_err(|error| *error), cx);
        result
    }
    /// Only the shared shortcut listener translates key chords into these actions.
    pub(crate) fn run_layout_action(
        &mut self,
        action: LayoutAction,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if self.route != crate::routes::Route::Session || self.areas.session.open.is_none() {
            return false;
        }
        let result = match action {
            LayoutAction::NewTab => match super::key(self) {
                Ok(workspace) => self
                    .layout_open_target(
                        &workspace,
                        Target::NewTab,
                        OpenIntent::New,
                        Placement::Focused,
                        window,
                        cx,
                    )
                    .map(|_| ()),
                Err(error) => Err(error),
            },
            LayoutAction::Split(position) => with_layout(self, |layout| {
                layout
                    .split(layout.focused_pane(), position, mode(window))
                    .map(|_| ())
            }),
            LayoutAction::Focus(direction) => {
                with_layout(self, |layout| layout.focus_neighbor(direction))
            }
            LayoutAction::Move(direction) => {
                with_layout(self, |layout| layout.move_neighbor(direction, mode(window)))
            }
            LayoutAction::StepTab(direction) => {
                with_layout(self, |layout| layout.step_tab(direction).map(|_| ()))
            }
            LayoutAction::JumpTab(index) => {
                with_layout(self, |layout| layout.jump_tab(index).map(|_| ()))
            }
            LayoutAction::Close(scope) => {
                let plans = with_layout(self, |layout| {
                    let tab = layout
                        .root()
                        .panes()
                        .into_iter()
                        .find(|pane| pane.id() == layout.focused_pane())
                        .and_then(Pane::focused);
                    tab.map_or_else(|| Ok(Vec::new()), |tab| layout.close_plans(tab, scope))
                });
                match plans {
                    Ok(plans) => self.layout_close_plans(plans, cx),
                    Err(error) => Err(error),
                }
            }
            LayoutAction::ClosePane => with_layout(self, |layout| {
                layout.close_empty_pane(layout.focused_pane())
            }),
            LayoutAction::ToggleFocus => {
                let pane = with_layout(self, |layout| Ok(layout.focused_pane()));
                match pane {
                    Ok(pane) => {
                        if let Ok(workspace) = super::key(self) {
                            let selected = (workspace, pane);
                            if self.areas.layout.maximized.as_ref() == Some(&selected) {
                                self.layout_exit_focus(cx);
                            } else {
                                self.areas.layout.maximized = Some(selected);
                            }
                        }
                        Ok(())
                    }
                    Err(error) => Err(error),
                }
            }
        };
        self.layout_result(result, cx);
        true
    }
    pub(super) fn layout_select(&mut self, tab: TabId, cx: &mut Context<'_, Self>) {
        let result = with_layout(self, |layout| layout.select(tab));
        self.layout_result(result, cx);
    }
    pub(super) fn layout_split(
        &mut self,
        pane: PaneId,
        position: Position,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) {
        let result = with_layout(self, |layout| {
            layout.split(pane, position, mode(window)).map(|_| ())
        });
        self.layout_result(result, cx);
    }
    pub(super) fn layout_drop(
        &mut self,
        tab: TabId,
        pane: PaneId,
        position: Position,
        window: &Window,
        cx: &mut Context<'_, Self>,
    ) {
        let result = with_layout(self, |layout| {
            layout.drop_tab(tab, pane, position, mode(window))
        });
        self.layout_result(result, cx);
    }
    /// Async owners call only after resource confirmation/RPC acknowledgement.
    pub(crate) fn layout_close_settled(
        &mut self,
        key: &vessel_app_view::layout::LayoutKey,
        plan: &ClosePlan,
        result: Result<(), LayoutError>,
        cx: &mut Context<'_, Self>,
    ) {
        super::initialized(self);
        let before = super::registry::retirement::Before::layout(self, key);
        let result = result.and_then(|()| {
            let mut book = self.areas.layout.book.borrow_mut();
            let layout = book
                .as_mut()
                .and_then(|book| book.get_mut(key))
                .ok_or(LayoutError::InvalidState)?;
            layout.close_confirmed(plan, CloseApproval::LifecycleSettled)
        });
        if result.is_ok() {
            self.layout_retired(before, super::registry::retirement::Source::Local, cx);
        }
        self.layout_result(result, cx);
    }
    fn layout_close_plans(
        &mut self,
        plans: Vec<ClosePlan>,
        cx: &mut Context<'_, Self>,
    ) -> Result<(), LayoutError> {
        if plans.iter().any(|plan| plan.effect() != CloseEffect::Local) {
            return Err(LayoutError::UnsupportedHost);
        }
        if plans.iter().any(|plan| plan.modified() == Modified::Dirty) {
            return Err(LayoutError::Modified);
        }
        let key = super::key(self)?;
        for plan in plans {
            self.layout_close_settled(&key, &plan, Ok(()), cx);
        }
        Ok(())
    }
}
