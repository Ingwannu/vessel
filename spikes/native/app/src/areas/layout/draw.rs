//! Pane projection reuses kit tabs and existing Explorer without copying resources.
use super::{key, mode, registry, with_layout};
use crate::shell::Shell;
use gpui::AppContext as _;
use vessel_app_kit::tabs::{tab, tab_head};
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    App, Context, Div, ElementId, Entity, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, Render, SharedString, StatefulInteractiveElement as _, Styled as _, Window,
    div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, button};
use vessel_app_view::layout::{
    AgentRole, Axis, Layout, LayoutMode, OpenIntent, Pane, PaneId, Placement, Position, SplitNode,
    TabId, Target,
};

#[derive(Clone)]
struct Dragged {
    tab: TabId,
    workspace: vessel_app_view::layout::LayoutKey,
    label: SharedString,
    tokens: Tokens,
}
struct DragPreview(Dragged);
impl Render for DragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        div()
            .px(px(spacing::S3))
            .py(px(spacing::S2))
            .bg(self.0.tokens.neutral.surface2)
            .text_color(self.0.tokens.neutral.foreground)
            .type_(Type::Body)
            .child(self.0.label.clone())
    }
}
/// Build the legacy agent body only after the registry admits its visible host.
pub(crate) fn frame(
    shell: &Shell,
    renderer: registry::AgentRenderer,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let Some(open) = shell.areas.session.open.as_ref() else {
        return div();
    };
    let state = with_layout(shell, |layout| {
        if !layout
            .tabs()
            .any(|tab| matches!(tab.target(),Target::Agent(id) if *id==open.session))
        {
            let _opened = layout.open(
                Target::Agent(open.session),
                OpenIntent::Reveal,
                Placement::Focused,
                AgentRole::Unknown,
                mode(window),
            )?;
        }
        Ok(layout.clone())
    });
    let (Ok(layout), Ok(key)) = (state, key(shell)) else {
        return div()
            .p(px(spacing::S4))
            .child(shell.words.layout().panel_unavailable().to_owned());
    };
    let mut agent = Some(renderer);
    let content = if let Some(pane) = super::queries::projected_pane(shell, &layout, &key, window)
        .and_then(|id| {
            layout
                .root()
                .panes()
                .into_iter()
                .find(|pane| pane.id() == id)
        }) {
        pane_view(
            (shell, &layout, &key),
            pane,
            &mut agent,
            tokens,
            (window, cx),
        )
    } else {
        tree(
            (shell, &layout, &key),
            layout.root(),
            &mut agent,
            tokens,
            (window, cx),
        )
    };
    let content = div()
        .size_full()
        .relative()
        .child(content)
        .children(super::sheets::overlay(
            shell, &layout, &key, &mut agent, tokens, window, cx,
        ))
        .children(super::menu::overlay(shell, tokens, window, cx))
        .children(super::header::menu_overlay(shell, tokens, window, cx))
        .children(super::rename::overlay(shell, tokens, window, cx))
        .children(shell.areas.layout.tips.render(tokens));
    crate::areas::workspace::code_beside(shell, content, tokens, window, cx)
}
type DrawSource<'a> = (
    &'a Shell,
    &'a Layout,
    &'a vessel_app_view::layout::LayoutKey,
);
type DrawUi<'a, 'cx> = (&'a mut Window, &'a mut Context<'cx, Shell>);
fn tree(
    source: DrawSource<'_>,
    node: &SplitNode,
    agent: &mut Option<registry::AgentRenderer>,
    tokens: &Tokens,
    ui: DrawUi<'_, '_>,
) -> Div {
    let (window, cx) = ui;
    match node {
        SplitNode::Pane(pane) => pane_view(source, pane, agent, tokens, (window, cx)),
        SplitNode::Group {
            axis,
            fraction,
            children: [a, b],
            id,
        } => {
            let first = tree(source, a, agent, tokens, (window, cx));
            let second = tree(source, b, agent, tokens, (window, cx));
            let group = div().size_full().flex().min_w_0().min_h_0();
            let group = match axis {
                Axis::Horizontal => group.flex_row(),
                Axis::Vertical => group.flex_col(),
            };
            let frame = group
                .child(
                    div()
                        .flex_basis(px(spacing::S0))
                        .flex_grow(fraction.get())
                        .min_w_0()
                        .min_h_0()
                        .child(first),
                )
                .child(
                    div()
                        .flex_basis(px(spacing::S0))
                        .flex_grow(fraction.get().mul_add(-1.0, 1.0))
                        .min_w_0()
                        .min_h_0()
                        .child(second),
                );
            super::resize::group(source.0, frame, *id, (*axis, fraction.get()), tokens, cx)
        }
    }
}
fn pane_view(
    source: DrawSource<'_>,
    pane: &Pane,
    agent: &mut Option<registry::AgentRenderer>,
    tokens: &Tokens,
    ui: DrawUi<'_, '_>,
) -> Div {
    let (shell, layout, workspace) = source;
    let (window, cx) = ui;
    let id = pane.id();
    let tabs = pane
        .tabs()
        .iter()
        .filter_map(|id| layout.tab(*id))
        .fold(div(), |row, tab| {
            row.child(tab_view(shell, tab, pane, workspace, tokens, cx))
        });
    let drop_key = workspace.clone();
    let controls = super::header::controls(shell, layout, pane, tokens, window, cx);
    let content = match pane.focused().and_then(|id| layout.tab(id)) {
        Some(tab) => registry::render(
            shell,
            &registry::PanelContext {
                workspace,
                pane: id,
                tab: tab.id(),
                target: tab.target(),
                focus: if layout.focused_pane() == id {
                    registry::PanelFocus::Focused
                } else {
                    registry::PanelFocus::Background
                },
            },
            agent,
            tokens,
            window,
            cx,
        ),
        None => launcher(shell, tokens, window, cx).into_any_element(),
    };
    let frame = div()
        .id(ElementId::Name(format!("layout-pane-{}", id.get()).into()))
        .size_full()
        .min_w_0()
        .min_h_0()
        .flex()
        .flex_col()
        .border_color(tokens.neutral.border)
        .border_r_1()
        .child(tab_head(tabs, Some(controls), tokens).h(px(
            if mode(window) == LayoutMode::Compact {
                vessel_app_kit::breakpoint::TOUCH_TARGET
            } else {
                vessel_app_kit::tabs::HEAD_HEIGHT
            },
        )))
        .child(div().flex_1().min_h_0().min_w_0().child(content))
        .children(drop_preview(shell, id, tokens))
        .on_drag_move::<Dragged>(cx.listener(
            move |shell, event: &gpui::DragMoveEvent<Dragged>, window, cx| {
                let x =
                    f32::from(event.bounds.left()).mul_add(-1.0, f32::from(event.event.position.x));
                let y =
                    f32::from(event.bounds.top()).mul_add(-1.0, f32::from(event.event.position.y));
                if let Ok(point) = vessel_app_view::layout::DropPoint::in_pane(
                    x,
                    y,
                    f32::from(event.bounds.size.width),
                    f32::from(event.bounds.size.height),
                ) {
                    let position = if mode(window) == LayoutMode::SplitPanes {
                        point.position()
                    } else {
                        Position::Center
                    };
                    if shell.areas.layout.drop_preview.get() != Some((id, position)) {
                        shell.areas.layout.drop_preview.set(Some((id, position)));
                        cx.notify();
                    }
                }
            },
        ))
        .on_drop(cx.listener(move |shell, dragged: &Dragged, window, cx| {
            if dragged.workspace != drop_key || key(shell).as_ref() != Ok(&drop_key) {
                shell.layout_result(Err(vessel_app_view::layout::LayoutError::InvalidState), cx);
                return;
            }
            let position = shell
                .areas
                .layout
                .drop_preview
                .get()
                .filter(|(pane, _)| *pane == id)
                .map_or(Position::Center, |(_, position)| position);
            shell.areas.layout.drop_preview.set(None);
            shell.layout_drop(dragged.tab, id, position, window, cx);
        }));
    div().size_full().child(frame)
}
fn tab_view(
    shell: &Shell,
    tab_state: &vessel_app_view::layout::Tab,
    pane: &Pane,
    workspace: &vessel_app_view::layout::LayoutKey,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> impl IntoElement {
    let id = tab_state.id();
    let label = tab_label(shell, tab_state.target());
    let dragged = Dragged {
        tab: id,
        workspace: workspace.clone(),
        label: label.clone().into(),
        tokens: *tokens,
    };
    let menu_key = workspace.clone();
    let select_key = workspace.clone();
    let drop_key = workspace.clone();
    tab(
        ElementId::Name(format!("layout-tab-{}", id.get()).into()),
        label,
        pane.focused() == Some(id),
        None,
        tokens,
    )
    .max_w(px(spacing::S32))
    .truncate()
    .on_click(cx.listener(move |shell, _event, _window, cx| {
        if key(shell).as_ref() == Ok(&select_key) {
            shell.layout_select(id, cx);
        }
    }))
    .on_mouse_down(
        MouseButton::Right,
        cx.listener(move |shell, event: &gpui::MouseDownEvent, window, cx| {
            shell.areas.layout.menu_tab = Some((menu_key.clone(), id));
            shell.areas.layout.menu.open(event.position, window, cx);
            cx.notify();
        }),
    )
    .on_drag_move::<Dragged>(cx.listener(
        move |shell, event: &gpui::DragMoveEvent<Dragged>, _window, cx| {
            let midpoint =
                f32::from(event.bounds.size.width).mul_add(0.5, f32::from(event.bounds.left()));
            let insertion = if f32::from(event.event.position.x) < midpoint {
                vessel_app_view::layout::TabInsertion::Before
            } else {
                vessel_app_view::layout::TabInsertion::After
            };
            if shell.areas.layout.tab_drop_preview.get() != Some((id, insertion)) {
                shell
                    .areas
                    .layout
                    .tab_drop_preview
                    .set(Some((id, insertion)));
                cx.notify();
            }
        },
    ))
    .on_drop(cx.listener(move |shell, source: &Dragged, window, cx| {
        cx.stop_propagation();
        if source.workspace != drop_key || key(shell).as_ref() != Ok(&drop_key) {
            return;
        }
        let insertion = shell
            .areas
            .layout
            .tab_drop_preview
            .get()
            .filter(|(tab, _)| *tab == id)
            .map_or(
                vessel_app_view::layout::TabInsertion::Before,
                |(_, insertion)| insertion,
            );
        shell.areas.layout.tab_drop_preview.set(None);
        shell.areas.layout.drop_preview.set(None);
        let result = super::with_layout_at(shell, &drop_key, |layout| {
            layout.drop_onto_tab(source.tab, id, insertion, mode(window))
        });
        shell.layout_result(result, cx);
    }))
    .on_drag(
        dragged,
        |dragged, _offset, _window, cx: &mut App| -> Entity<DragPreview> {
            cx.new(|_| DragPreview(dragged.clone()))
        },
    )
}
fn tab_label(shell: &Shell, target: &Target) -> String {
    match target {
        Target::Agent(session) => shell
            .areas
            .session
            .open
            .as_ref()
            .filter(|open| open.session == *session)
            .map_or_else(
                || session.to_string(),
                |open| {
                    if open.title.trim().is_empty() {
                        shell.t.untitled.to_owned()
                    } else {
                        open.title.clone()
                    }
                },
            ),
        Target::NewTab => shell.words.layout().new_tab().to_owned(),
        Target::File { path, .. } => path.as_str().to_owned(),
        Target::Terminal(id) => id.to_string(),
        Target::Browser(id) => id.as_str().to_owned(),
        Target::Draft(id) => id.to_string(),
        Target::Changes => shell.words.layout().changes().to_owned(),
        Target::Files => shell.words.layout().files().to_owned(),
        Target::Scripts
        | Target::PullRequest
        | Target::WorkingDiff
        | Target::ProviderSubagent { .. }
        | Target::Plugin { .. }
        | Target::Setup(_)
        | Target::CommitDiff(_) => shell.words.layout().tabs().to_owned(),
    }
}

pub(super) fn launcher(
    shell: &Shell,
    tokens: &Tokens,
    _window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Div {
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(spacing::S3))
        .p(px(spacing::S4))
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground_muted)
                .child(shell.words.layout().empty().to_owned()),
        )
        .child(
            button(
                "layout-launch-agent",
                shell.words.layout().new_agent().to_owned(),
                false,
                tokens,
            )
            .on_click(cx.listener(|shell, _event, window, cx| {
                shell.go(crate::routes::Route::Draft, window, cx);
            })),
        )
}

fn drop_preview(shell: &Shell, pane: PaneId, tokens: &Tokens) -> Option<Div> {
    let (target, position) = shell.areas.layout.drop_preview.get()?;
    if target != pane {
        return None;
    }
    let frame = div()
        .absolute()
        .border_1()
        .border_color(tokens.neutral.foreground_muted);
    Some(match position {
        Position::Center => frame.inset_0(),
        Position::Left => frame.top_0().bottom_0().left_0().w(gpui::relative(0.5)),
        Position::Right => frame.top_0().bottom_0().right_0().w(gpui::relative(0.5)),
        Position::Top => frame.top_0().left_0().right_0().h(gpui::relative(0.5)),
        Position::Bottom => frame.bottom_0().left_0().right_0().h(gpui::relative(0.5)),
    })
}
