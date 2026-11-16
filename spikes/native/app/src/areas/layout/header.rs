//! Pane controls consume the one source toolbar chrome and canonical UI glyphs.
use super::{mode, with_layout};
use crate::shell::Shell;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::toolbar_button::{
    Selection, ToolbarButton, ToolbarConfig, ToolbarIcon, ToolbarInk,
};
use vessel_app_kit::ui::{Context, Div, ElementId, ParentElement as _, Window, div};
use vessel_app_kit::{ButtonState, ControlSizing, Tokens};
use vessel_app_view::layout::{
    Layout, LayoutAction, LayoutActionContext, LayoutMode, OpenIntent, Pane, PaneId, Placement,
    Position, Target,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Control {
    New,
    Right,
    Down,
    Focus,
    Close,
}
impl Control {
    fn glyph(self, maximized: bool) -> Glyph {
        match self {
            Self::New => Glyph::Plus,
            Self::Right => Glyph::Columns2,
            Self::Down => Glyph::Rows2,
            Self::Focus if maximized => Glyph::PanelMinimize,
            Self::Focus => Glyph::PanelMaximize,
            Self::Close => Glyph::X,
        }
    }
    fn label(self, shell: &Shell, maximized: bool) -> &str {
        let words = shell.words.layout();
        match self {
            Self::New => words.new_tab(),
            Self::Right => words.split_right(),
            Self::Down => words.split_down(),
            Self::Focus if maximized => words.restore(),
            Self::Focus => words.maximize(),
            Self::Close => words.close_pane(),
        }
    }
}
pub(super) fn controls(
    shell: &Shell,
    layout: &Layout,
    pane: &Pane,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let mut row = div();
    let mut controls = vec![Control::New];
    if mode(window) == LayoutMode::SplitPanes {
        controls.extend([Control::Right, Control::Down]);
        if layout.root().panes().len() > 1 {
            controls.push(Control::Focus);
        }
    }
    if pane.tabs().is_empty() {
        controls.push(Control::Close);
    }
    let Ok(workspace) = super::key(shell) else {
        return row;
    };
    let Some(context) = shell.layout_action_context() else {
        return row;
    };
    let pane_id = pane.id();
    let maximized = shell
        .areas
        .layout
        .maximized
        .as_ref()
        .is_some_and(|(key, pane)| *key == workspace && *pane == pane_id);
    let sizing = ControlSizing::from_compact(mode(window) == LayoutMode::Compact);
    for control in controls {
        let action = PaneAction {
            control,
            pane: pane_id,
            context: context.clone(),
        };
        let config = ToolbarConfig::new(sizing, ButtonState::Enabled).selected(
            if control == Control::Focus && maximized {
                Selection::Selected
            } else {
                Selection::Unselected
            },
        );
        let button = ToolbarButton::new(
            ElementId::Name(format!("layout-control-{}-{control:?}", pane_id.get()).into()),
            control.label(shell, maximized).to_owned(),
            ToolbarIcon::new(control.glyph(maximized), ToolbarInk::ExtraMuted),
            config,
            tokens,
        )
        .on_click(cx.listener(move |shell, _event, window, cx| {
            action.run(shell, window, cx);
        }));
        let place = std::rc::Rc::clone(
            shell
                .areas
                .layout
                .tip_places
                .borrow_mut()
                .entry((pane_id, control))
                .or_insert_with(|| std::rc::Rc::new(std::cell::Cell::new(None))),
        );
        row = row.child(button.tooltip(&shell.areas.layout.tips, &place));
    }
    row
}

// Keep the captured owner selection rather than following whatever tab happens
// to be selected when a delayed callback reaches the event queue.
struct PaneAction {
    control: Control,
    pane: PaneId,
    context: LayoutActionContext,
}
impl PaneAction {
    fn run(&self, shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
        if !shell.layout_action_context_is_current(&self.context) {
            return;
        }
        let pane_id = self.pane;
        match self.control {
            Control::New => {
                let result = shell
                    .layout_open_target(
                        self.context.workspace(),
                        Target::NewTab,
                        OpenIntent::New,
                        Placement::Pane(pane_id),
                        window,
                        cx,
                    )
                    .map(|_| ());
                shell.layout_result(result, cx);
            }
            Control::Right => shell.layout_split(pane_id, Position::Right, window, cx),
            Control::Down => shell.layout_split(pane_id, Position::Bottom, window, cx),
            Control::Focus => {
                let result = with_layout(shell, |layout| layout.focus(pane_id));
                if result.is_ok() {
                    if let Some(context) = shell.layout_action_context() {
                        let _handled = shell.run_layout_action_at(
                            &context,
                            LayoutAction::ToggleFocus,
                            window,
                            cx,
                        );
                    }
                } else {
                    shell.layout_result(result, cx);
                }
            }
            Control::Close => {
                let result = with_layout(shell, |layout| layout.close_empty_pane(pane_id));
                shell.layout_result(result, cx);
            }
        }
    }
}

#[cfg(test)]
mod tests;
