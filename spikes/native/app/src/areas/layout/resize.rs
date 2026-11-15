//! Split resizing uses GPUI drag events in both axes and saves only on release.
use super::with_layout;
use crate::shell::Shell;
use gpui::AppContext as _;
use vessel_app_kit::Tokens;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    App, Context, Div, ElementId, Entity, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, Render, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_view::layout::{Axis, GroupId};

#[derive(Clone)]
struct Drag {
    group: GroupId,
    workspace: vessel_app_view::layout::LayoutKey,
    axis: Axis,
}
struct Preview;
impl Render for Preview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        div()
    }
}
/// One group receives only the drag whose stable id belongs to it.
pub(super) fn group(
    shell: &Shell,
    frame: Div,
    id: GroupId,
    geometry: (Axis, f32),
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let (axis, share) = geometry;
    let Ok(workspace) = super::key(shell) else {
        return frame;
    };
    let resizing = std::rc::Rc::clone(&shell.areas.layout.resizing);
    let handle = div()
        .id(ElementId::Name(format!("split-handle-{}", id.get()).into()))
        .absolute()
        .bg(tokens.neutral.border)
        .on_drag(
            Drag {
                group: id,
                axis,
                workspace,
            },
            move |_drag, _offset, _window, cx: &mut App| -> Entity<Preview> {
                resizing.set(Some(id));
                cx.new(|_| Preview)
            },
        );
    let handle = match axis {
        Axis::Horizontal => handle
            .top_0()
            .bottom_0()
            .left(gpui::relative(share))
            .w(px(spacing::S1))
            .cursor_col_resize(),
        Axis::Vertical => handle
            .left_0()
            .right_0()
            .top(gpui::relative(share))
            .h(px(spacing::S1))
            .cursor_row_resize(),
    };
    // Capture release even outside the group, so disk writes follow the gesture once.
    let group = frame
        .id(ElementId::Name(format!("split-group-{}", id.get()).into()))
        .relative()
        .child(handle)
        .on_drag_move::<Drag>(cx.listener(
            move |shell, event: &gpui::DragMoveEvent<Drag>, _window, cx| {
                let drag = event.drag(cx).clone();
                if drag.group != id || super::key(shell).as_ref() != Ok(&drag.workspace) {
                    return;
                }
                let (pointer, origin, room) = match drag.axis {
                    Axis::Horizontal => (
                        f32::from(event.event.position.x),
                        f32::from(event.bounds.left()),
                        f32::from(event.bounds.size.width),
                    ),
                    Axis::Vertical => (
                        f32::from(event.event.position.y),
                        f32::from(event.bounds.top()),
                        f32::from(event.bounds.size.height),
                    ),
                };
                if room > 0.0 {
                    let fraction = origin.mul_add(-1.0, pointer).mul_add(room.recip(), 0.0);
                    let result = with_layout(shell, |layout| layout.resize(id, fraction));
                    if result.is_ok() {
                        cx.notify();
                    }
                }
            },
        ))
        .capture_any_mouse_up(cx.listener(
            move |shell, event: &gpui::MouseUpEvent, _window, cx| {
                if event.button == MouseButton::Left
                    && shell.areas.layout.resizing.get() == Some(id)
                {
                    shell.areas.layout.resizing.set(None);
                    shell.layout_result(Ok(()), cx);
                }
            },
        ));
    div().size_full().child(group)
}
