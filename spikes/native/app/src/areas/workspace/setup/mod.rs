//! Setup panes are passive until the canonical registry/runtime adapter exists.
//! A workspace id or historical worktree success cannot authorize cancellation.
mod draw;
use crate::areas::layout::registry::PanelContext;
use crate::shell::Shell;
use vessel_app_kit::Tokens;
use vessel_app_kit::ui::{AnyElement, Context, IntoElement as _, Window, div};
use vessel_app_view::layout::{LayoutKey, Target};
use vessel_app_view::workspace::setup_observation::Model;

pub(crate) fn render_panel(
    shell: &Shell,
    panel: &PanelContext<'_>,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let (LayoutKey::Workspace { workspace, .. }, Target::Setup(target)) =
        (panel.workspace, panel.target)
    else {
        return div().into_any_element();
    };
    if workspace != target
        || !shell.layout_tab_matches(panel.workspace, panel.tab, panel.target)
        || !shell.layout_panel_visible(panel.workspace, panel.tab, window)
    {
        return div().into_any_element();
    }
    // No canonical workspace-to-backing admission producer is installed.
    // Creating no address, task or capability avoids making old worktree
    // history a cancellation/start grant. The default model is unavailable.
    draw::render(
        &draw::Card {
            shell,
            model: &Model::default(),
            dispatch: None,
            tokens,
        },
        window,
        cx,
    )
    .into_any_element()
}

#[cfg(test)]
mod tests;
