//! Reference geometry, independent of section/row delegation: Paseo's
//! settings-section.tsx and styles/settings.ts, plus the kit's unchanged
//! 16px Meta / 21px Body line steps.
use super::*;
use crate::theme::Scheme;
use gpui::{Bounds, Context, IntoElement, Pixels, Render, TestAppContext, Window, size};
use std::{cell::RefCell, rc::Rc};
type Seen = Rc<RefCell<Vec<Bounds<Pixels>>>>;
#[derive(Clone, Copy)]
enum SectionKind {
    Legacy,
    EmptySlot,
    Action,
}
struct SectionProbe {
    kind: SectionKind,
    inner: Seen,
    outer: Seen,
    action: Seen,
}
impl Render for SectionProbe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let card = div().h(px(80.0));
        let action = Rc::clone(&self.action);
        let trailing = div()
            .size(px(28.0))
            .on_children_prepainted(move |bounds, _, _| *action.borrow_mut() = bounds)
            .child(div().size_full())
            .into_any_element();
        let section = match self.kind {
            SectionKind::Legacy => section("Agents", card, &tokens),
            SectionKind::EmptySlot => section_with_trailing("Agents", card, None, &tokens),
            SectionKind::Action => section_with_trailing("Agents", card, Some(trailing), &tokens),
        };
        let inner = Rc::clone(&self.inner);
        let outer = Rc::clone(&self.outer);
        div()
            .w(px(360.0))
            .flex()
            .flex_col()
            .on_children_prepainted(move |bounds, _, _| *outer.borrow_mut() = bounds)
            .child(section.on_children_prepainted(move |bounds, _, _| *inner.borrow_mut() = bounds))
            .child(div().h(px(1.0)))
    }
}
fn bounds(x: f32, y: f32, w: f32, h: f32) -> Bounds<Pixels> {
    Bounds::new(point(px(x), px(y)), size(px(w), px(h)))
}
struct SectionBounds {
    head_and_card: Vec<Bounds<Pixels>>,
    section_and_tail: Vec<Bounds<Pixels>>,
    action: Vec<Bounds<Pixels>>,
}
fn section_measure(cx: &mut TestAppContext, kind: SectionKind) -> SectionBounds {
    let inner = Seen::default();
    let outer = Seen::default();
    let action = Seen::default();
    let (_, cx) = cx.add_window_view({
        let inner = Rc::clone(&inner);
        let outer = Rc::clone(&outer);
        let action = Rc::clone(&action);
        move |_, _| SectionProbe {
            kind,
            inner,
            outer,
            action,
        }
    });
    cx.run_until_parked();
    SectionBounds {
        head_and_card: inner.borrow().clone(),
        section_and_tail: outer.borrow().clone(),
        action: action.borrow().clone(),
    }
}
#[gpui::test]
fn section_bounds_match_fixed_reference_geometry_with_and_without_an_action(
    cx: &mut TestAppContext,
) {
    for kind in [SectionKind::Legacy, SectionKind::EmptySlot] {
        let SectionBounds {
            head_and_card: inner,
            section_and_tail: outer,
            action,
        } = section_measure(cx, kind);
        assert_eq!(
            inner,
            vec![
                bounds(4.0, 0.0, 356.0, 16.0),
                bounds(0.0, 28.0, 360.0, 80.0)
            ]
        );
        assert_eq!(
            outer,
            vec![
                bounds(0.0, 0.0, 360.0, 108.0),
                bounds(0.0, 132.0, 360.0, 1.0)
            ]
        );
        assert!(action.is_empty());
    }
    let SectionBounds {
        head_and_card: inner,
        section_and_tail: outer,
        action,
    } = section_measure(cx, SectionKind::Action);
    assert_eq!(
        inner,
        vec![
            bounds(4.0, 0.0, 356.0, 28.0),
            bounds(0.0, 40.0, 360.0, 80.0)
        ]
    );
    assert_eq!(
        outer,
        vec![
            bounds(0.0, 0.0, 360.0, 120.0),
            bounds(0.0, 144.0, 360.0, 1.0)
        ]
    );
    assert_eq!(action, vec![bounds(332.0, 0.0, 28.0, 28.0)]);
}
#[derive(Clone, Copy)]
enum RowKind {
    Legacy,
    Custom,
}
struct RowProbe {
    kind: RowKind,
    first: bool,
    compact: bool,
    inner: Seen,
    outer: Seen,
}
impl Render for RowProbe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let control = div()
            .w(px(if self.compact { 220.0 } else { 28.0 }))
            .h(px(28.0))
            .into_any_element();
        let row = match self.kind {
            RowKind::Legacy => row_at(
                "Title",
                None,
                None,
                Some(control),
                self.first,
                self.compact,
                &tokens,
            ),
            RowKind::Custom => row_with_content(
                div()
                    .flex_1()
                    .min_w_0()
                    .when(self.compact, |content| content.flex_basis(px(160.0)))
                    .h(px(21.0))
                    .into_any_element(),
                Some(control),
                self.first,
                self.compact,
                &tokens,
            ),
        };
        let inner = Rc::clone(&self.inner);
        let outer = Rc::clone(&self.outer);
        div()
            .w(px(360.0))
            .flex()
            .flex_col()
            .on_children_prepainted(move |bounds, _, _| *outer.borrow_mut() = bounds)
            .child(row.on_children_prepainted(move |bounds, _, _| *inner.borrow_mut() = bounds))
    }
}
fn row_measure(
    cx: &mut TestAppContext,
    kind: RowKind,
    first: bool,
    compact: bool,
) -> (Vec<Bounds<Pixels>>, Vec<Bounds<Pixels>>) {
    let inner = Seen::default();
    let outer = Seen::default();
    let (_, cx) = cx.add_window_view({
        let inner = Rc::clone(&inner);
        let outer = Rc::clone(&outer);
        move |_, _| RowProbe {
            kind,
            first,
            compact,
            inner,
            outer,
        }
    });
    cx.run_until_parked();
    (inner.borrow().clone(), outer.borrow().clone())
}
#[gpui::test]
fn custom_and_existing_rows_keep_pointer_insets_control_gap_and_hairline(cx: &mut TestAppContext) {
    for kind in [RowKind::Legacy, RowKind::Custom] {
        let (inner, outer) = row_measure(cx, kind, true, false);
        assert_eq!(
            inner,
            vec![
                bounds(16.0, 19.5, 288.0, 21.0),
                bounds(316.0, 16.0, 28.0, 28.0)
            ]
        );
        assert_eq!(outer, vec![bounds(0.0, 0.0, 360.0, 60.0)]);
        let (inner, outer) = row_measure(cx, kind, false, false);
        assert_eq!(
            inner,
            vec![
                bounds(16.0, 20.5, 288.0, 21.0),
                bounds(316.0, 17.0, 28.0, 28.0)
            ]
        );
        assert_eq!(outer, vec![bounds(0.0, 0.0, 360.0, 61.0)]);
    }
}
#[gpui::test]
fn custom_and_existing_compact_rows_wrap_with_the_reference_gap(cx: &mut TestAppContext) {
    for kind in [RowKind::Legacy, RowKind::Custom] {
        let (inner, outer) = row_measure(cx, kind, true, true);
        assert_eq!(
            inner,
            vec![
                bounds(16.0, 16.0, 328.0, 21.0),
                bounds(16.0, 49.0, 220.0, 28.0)
            ]
        );
        assert_eq!(outer, vec![bounds(0.0, 0.0, 360.0, 93.0)]);
    }
}
