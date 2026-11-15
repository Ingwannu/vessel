use super::*;
use crate::Scheme;
use gpui::{Bounds, Modifiers, Pixels, Render, TestAppContext, point, size};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
type Place = Rc<Cell<Option<Bounds<Pixels>>>>;
type Seen = Rc<RefCell<Vec<Bounds<Pixels>>>>;
struct Probe {
    density: Density,
    indent: Indent,
    scheme: Scheme,
    meta: bool,
    leading: Place,
    metadata: Place,
    outer: Seen,
    presses: Rc<Cell<usize>>,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(self.scheme);
        let presses = Rc::clone(&self.presses);
        let outer = Rc::clone(&self.outer);
        let mut row = SidebarWorkspaceRow::new("workspace", "Feature workspace")
            .indent(self.indent)
            .leading(
                div()
                    .relative()
                    .size_full()
                    .child(crate::picker::anchor_probe(&self.leading))
                    .into_any_element(),
            );
        if self.meta {
            row = row.metadata(
                div()
                    .relative()
                    .h(px(16.0))
                    .child(crate::picker::anchor_probe(&self.metadata))
                    .into_any_element(),
            );
        }
        div()
            .w(px(320.0))
            .flex()
            .flex_col()
            .on_children_prepainted(move |bounds, _, _| *outer.borrow_mut() = bounds)
            .child(row.element(self.density, &tokens, move |_, _, _| {
                presses.set(presses.get().saturating_add(1));
            }))
            .child(div().h(px(1.0)))
    }
}
fn bounds(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
    Bounds::new(point(px(x), px(y)), size(px(w), px(h)))
}
#[derive(Clone, Copy)]
struct GeometryCase {
    density: Density,
    indent: Indent,
    leading: Bounds<Pixels>,
    row: Bounds<Pixels>,
    after: Bounds<Pixels>,
    metadata: Option<Bounds<Pixels>>,
}
fn geometry_cases() -> [GeometryCase; 6] {
    [
        GeometryCase {
            density: Density::Desktop,
            indent: Indent::Flush,
            leading: bounds(8.0, 8.0, 16.0, 20.0),
            row: bounds(0.0, 0.0, 320.0, 36.0),
            after: bounds(0.0, 40.0, 320.0, 1.0),
            metadata: None,
        },
        GeometryCase {
            density: Density::Desktop,
            indent: Indent::Grouped,
            leading: bounds(16.0, 8.0, 16.0, 20.0),
            row: bounds(0.0, 0.0, 320.0, 36.0),
            after: bounds(0.0, 40.0, 320.0, 1.0),
            metadata: None,
        },
        GeometryCase {
            density: Density::Compact,
            indent: Indent::Flush,
            leading: bounds(8.0, 12.0, 20.0, 20.0),
            row: bounds(0.0, 0.0, 320.0, 44.0),
            after: bounds(0.0, 48.0, 320.0, 1.0),
            metadata: None,
        },
        GeometryCase {
            density: Density::Compact,
            indent: Indent::Grouped,
            leading: bounds(16.0, 12.0, 20.0, 20.0),
            row: bounds(0.0, 0.0, 320.0, 44.0),
            after: bounds(0.0, 48.0, 320.0, 1.0),
            metadata: None,
        },
        GeometryCase {
            density: Density::Desktop,
            indent: Indent::Grouped,
            leading: bounds(16.0, 8.0, 16.0, 20.0),
            row: bounds(0.0, 0.0, 320.0, 52.0),
            after: bounds(0.0, 56.0, 320.0, 1.0),
            metadata: Some(bounds(40.0, 28.0, 268.0, 16.0)),
        },
        GeometryCase {
            density: Density::Compact,
            indent: Indent::Grouped,
            leading: bounds(16.0, 8.0, 20.0, 20.0),
            row: bounds(0.0, 0.0, 320.0, 52.0),
            after: bounds(0.0, 56.0, 320.0, 1.0),
            metadata: Some(bounds(44.0, 28.0, 264.0, 16.0)),
        },
    ]
}
#[gpui::test]
fn workspace_row_source_geometry_uses_one_rail_for_each_group_and_both_themes(
    cx: &mut TestAppContext,
) {
    for scheme in [Scheme::Dark, Scheme::Light] {
        for case in geometry_cases() {
            let leading = Place::default();
            let metadata = Place::default();
            let outer = Seen::default();
            let presses = Rc::new(Cell::new(0));
            let (_, cx) = cx.add_window_view({
                let leading = Rc::clone(&leading);
                let metadata = Rc::clone(&metadata);
                let outer = Rc::clone(&outer);
                let presses = Rc::clone(&presses);
                move |_, _| Probe {
                    density: case.density,
                    indent: case.indent,
                    scheme,
                    meta: case.metadata.is_some(),
                    leading,
                    metadata,
                    outer,
                    presses,
                }
            });
            cx.run_until_parked();
            assert_eq!(leading.get(), Some(case.leading));
            assert_eq!(*outer.borrow(), [case.row, case.after]);
            assert_eq!(metadata.get(), case.metadata);
            cx.simulate_click(point(px(200.0), px(18.0)), Modifiers::none());
            cx.run_until_parked();
            assert_eq!(presses.get(), 1);
        }
    }
}
#[test]
fn workspace_row_selected_hover_and_drag_backdrops_match_the_source_precedence() {
    for scheme in [Scheme::Dark, Scheme::Light] {
        let tokens = Tokens::of(scheme);
        let (hover, selected) = match scheme {
            Scheme::Light => (tokens.neutral.surface3, tokens.neutral.surface4),
            Scheme::Dark => (tokens.neutral.surface1, tokens.neutral.surface2),
        };
        let idle = Paints::new(Selection::Inactive, DragState::Resting, &tokens);
        assert_eq!(idle.rest, tokens.neutral.surface_sidebar);
        assert_eq!(idle.hover, hover);
        let active = Paints::new(Selection::Active, DragState::Resting, &tokens);
        assert_eq!(active.rest, selected);
        assert_eq!(active.hover, selected);
        for selection in [Selection::Inactive, Selection::Active] {
            let drag = Paints::new(selection, DragState::Dragging, &tokens);
            assert_eq!(drag.rest, tokens.neutral.surface2);
            assert_eq!(drag.hover, tokens.neutral.surface2);
            assert_eq!(drag.pressed, tokens.neutral.surface2);
        }
    }
}

struct DragProbe;
struct DragGhost;
impl Render for DragGhost {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        gpui::Empty
    }
}
impl Render for DragProbe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        use gpui::AppContext as _;
        SidebarWorkspaceRow::new("workspace-drag", "Workspace")
            .element(Density::Desktop, &Tokens::of(Scheme::Dark), |_, _, _| {})
            .w(px(320.0))
            .on_drag((), |(), _, _, cx| cx.new(|_| DragGhost))
    }
}
#[gpui::test]
fn workspace_row_uses_the_shared_six_point_drag_threshold(cx: &mut TestAppContext) {
    for (distance, expected) in [(5.0_f32, false), (7.0_f32, true)] {
        let (_, cx) = cx.add_window_view(|_, _| DragProbe);
        cx.run_until_parked();
        cx.simulate_mouse_down(
            point(px(20.0), px(18.0)),
            gpui::MouseButton::Left,
            Modifiers::none(),
        );
        cx.simulate_mouse_move(
            point(px(distance.mul_add(1.0, 20.0)), px(18.0)),
            gpui::MouseButton::Left,
            Modifiers::none(),
        );
        assert_eq!(cx.update(|_, cx| cx.has_active_drag()), expected);
    }
}
