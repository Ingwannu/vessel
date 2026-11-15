//! One typed panel registration point. Resource state belongs to the owning area.
pub(crate) mod retirement;
use crate::shell::Shell;
use vessel_app_kit::Tokens;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, InteractiveElement, IntoElement,
    ParentElement as _, Styled as _, Window, div, px,
};
use vessel_app_view::layout::{LayoutKey, PaneId, TabId, Target};

/// Focus is supplied by the host; hidden panes never claim terminal input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PanelFocus {
    Focused,
    Background,
}
/// The single immutable adapter context passed to feature-owned renderers.
pub(crate) struct PanelContext<'a> {
    /// Logical ownership, independent from the selected session or cwd.
    pub(crate) workspace: &'a LayoutKey,
    /// Current host pane; moving does not change resource identity.
    pub(crate) pane: PaneId,
    /// Stable presentation identity across moves/sheets.
    pub(crate) tab: TabId,
    /// Feature-owned target, fully typed.
    pub(crate) target: &'a Target,
    /// Whether it may claim keyboard/view focus.
    pub(crate) focus: PanelFocus,
}
/// Existing agent screen is consumed once; other resource renderers register here.
pub(super) fn render(
    shell: &Shell,
    panel: &PanelContext<'_>,
    agent: &mut Option<Div>,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    if !shell.layout_tab_matches(panel.workspace, panel.tab, panel.target)
        || !shell.layout_panel_visible(panel.workspace, panel.tab, window)
    {
        return div().into_any_element();
    }
    let body = match panel.target {
        Target::Agent(session)
            if super::key(shell).as_ref() == Ok(panel.workspace)
                && shell
                    .areas
                    .session
                    .open
                    .as_ref()
                    .is_some_and(|open| open.session == *session) =>
        {
            agent.take().unwrap_or_else(div).into_any_element()
        }
        Target::NewTab => super::draw::launcher(shell, tokens, window, cx).into_any_element(),
        Target::Agent(_)
        | Target::Draft(_)
        | Target::ProviderSubagent { .. }
        | Target::Terminal(_)
        | Target::Browser(_)
        | Target::Changes
        | Target::Files
        | Target::Scripts
        | Target::PullRequest
        | Target::File { .. }
        | Target::WorkingDiff
        | Target::Plugin { .. }
        | Target::Setup(_)
        | Target::CommitDiff(_) => div()
            .p(px(spacing::S4))
            .child(shell.words.layout().panel_unavailable().to_owned())
            .into_any_element(),
    };
    div()
        .id(vessel_app_kit::ui::ElementId::Name(
            format!(
                "panel-{:?}-{}-{}",
                panel.workspace,
                panel.pane.get(),
                panel.tab.get()
            )
            .into(),
        ))
        .size_full()
        .min_w_0()
        .min_h_0()
        .when(
            panel.focus == PanelFocus::Focused,
            InteractiveElement::occlude,
        )
        .child(body)
        .into_any_element()
}
