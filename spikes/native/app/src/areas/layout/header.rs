//! Pane controls reuse kit ghost buttons and tooltips, including touch-sized targets.
use super::{mode, with_layout};
use crate::shell::Shell;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    Context, Div, ElementId, ParentElement as _, StatefulInteractiveElement as _, Styled as _,
    Window, div, px,
};
use vessel_app_kit::{ButtonKind, ButtonState, Tokens, button_with_kind, glyph};
use vessel_app_view::layout::{
    LayoutAction, LayoutMode, OpenIntent, Pane, Placement, Position, Target,
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
    fn glyph(self) -> Glyph {
        match self {
            Self::New => Glyph::Plus,
            Self::Right => Glyph::PanelRight,
            Self::Down => Glyph::Split,
            Self::Focus => Glyph::Maximize,
            Self::Close => Glyph::Close,
        }
    }
    fn label(self, shell: &Shell) -> &str {
        let words = shell.words.layout();
        match self {
            Self::New => words.new_tab(),
            Self::Right => words.split_right(),
            Self::Down => words.split_down(),
            Self::Focus => words.maximize(),
            Self::Close => words.close_pane(),
        }
    }
}
pub(super) fn controls(
    shell: &Shell,
    pane: &Pane,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let mut row = div();
    let mut controls = vec![Control::New];
    if mode(window) == LayoutMode::SplitPanes {
        controls.extend([Control::Right, Control::Down, Control::Focus]);
    }
    if pane.tabs().is_empty() {
        controls.push(Control::Close);
    }
    let pane_id = pane.id();
    for control in controls {
        let Ok(workspace) = super::key(shell) else {
            continue;
        };
        let Some(context) = shell.layout_action_context() else {
            continue;
        };
        let touch = mode(window) == LayoutMode::Compact;
        let side = if touch {
            vessel_app_kit::breakpoint::TOUCH_TARGET
        } else {
            vessel_app_kit::tabs::HEAD_HEIGHT
        };
        let button = button_with_kind(
            ElementId::Name(format!("layout-control-{}-{control:?}", pane_id.get()).into()),
            "",
            ButtonKind::Ghost,
            ButtonState::Enabled,
            tokens,
        )
        .w(px(side))
        .h(px(side))
        .p(px(spacing::S0))
        .child(glyph(
            control.glyph(),
            spacing::S4,
            tokens.neutral.foreground_muted,
        ))
        .on_click(cx.listener(move |shell, _event, window, cx| {
            if !shell.layout_action_context_is_current(&context) {
                return;
            }
            match control {
                Control::New => {
                    let result = shell
                        .layout_open_target(
                            &workspace,
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
        row = row.child(shell.areas.layout.tips.on(
            button,
            &place,
            control.label(shell).to_owned(),
            touch,
        ));
    }
    row
}
