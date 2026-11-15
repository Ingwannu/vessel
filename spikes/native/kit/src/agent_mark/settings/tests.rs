//! Geometry through the unchanged shared settings frame, compared with
//! its previous `Chevron14`/`Bot16` contents; no application code is imported.
use super::*;
use crate::{Type, TypeExt as _, assets::Glyph, glyph, settings};
use gpui::{Bounds, Context, IntoElement, Pixels, Render, TestAppContext, Window, size};
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
struct Measurements {
    rows: Vec<Bounds<Pixels>>,
    slots: Vec<Vec<Bounds<Pixels>>>,
    images: Vec<Vec<Bounds<Pixels>>>,
    titles: Vec<Vec<Bounds<Pixels>>>,
}
impl Measurements {
    fn empty() -> Self {
        Self {
            rows: Vec::new(),
            slots: vec![Vec::new(); 6],
            images: vec![Vec::new(); 6],
            titles: vec![Vec::new(); 6],
        }
    }
}
struct Probe {
    width: f32,
    marked: bool,
    errors: bool,
    tokens: Tokens,
    seen: Rc<RefCell<Measurements>>,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = self.tokens;
        let compact = self.width < 600.0;
        let mut card = settings::card(&tokens);
        for (index, art) in [
            Some(OPENCODE),
            Some(super::super::CLAUDE),
            Some(super::super::CODEX),
            Some(super::super::GEMINI),
            None,
            Some(OPENCODE),
        ]
        .into_iter()
        .enumerate()
        {
            let edge = if index == 0 {
                SettingsRowEdge::First
            } else {
                SettingsRowEdge::Following
            };
            let title = self.title(index, compact, &tokens);
            let column = div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(title)
                .when(self.errors && !compact && art == Some(OPENCODE), |column| {
                    column.child(div().mt(px(4.0)).type_(Type::Meta).text_color(tokens.status().red_300).line_clamp(3).child(
                        "The agent could not be checked. 에이전트 상태를 확인할 수 없습니다.",
                    ))
                });
            let mark = if self.marked {
                let seen = Rc::clone(&self.seen);
                settings_mark(art, Size::Sixteen, edge, &tokens)
                    .on_children_prepainted(move |bounds, _, _| {
                        seen.borrow_mut().images[index] = bounds;
                    })
                    .into_any_element()
            } else {
                glyph(Glyph::Bot, 16.0, tokens.neutral.foreground).into_any_element()
            };
            let seen = Rc::clone(&self.seen);
            let contents = div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap(px(8.0))
                .on_children_prepainted(move |bounds, _, _| seen.borrow_mut().slots[index] = bounds)
                .child(glyph(
                    Glyph::ChevronRight,
                    14.0,
                    tokens.neutral.foreground_muted,
                ))
                .child(mark)
                .child(column);
            card = card.child(
                settings::row_with_content(
                    contents.into_any_element(),
                    None,
                    index == 0,
                    compact,
                    &tokens,
                )
                .min_h(px(56.0)),
            );
        }
        let seen = Rc::clone(&self.seen);
        div()
            .w(px(self.width))
            .max_w(px(720.0))
            .p(px(16.0))
            .type_(Type::Body)
            .child(card.on_children_prepainted(move |bounds, _, _| seen.borrow_mut().rows = bounds))
    }
}
impl Probe {
    fn title(&self, index: usize, compact: bool, tokens: &Tokens) -> Div {
        let seen = Rc::clone(&self.seen);
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .on_children_prepainted(move |bounds, _, _| seen.borrow_mut().titles[index] = bounds)
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .type_(Type::Body)
                    .child("Long authentication lifecycle · 저장하지 않은 편집 내용과 상태 복구"),
            )
            .when(!compact, |title| {
                title.child(div().type_(Type::Body).child("·"))
            })
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .type_(Type::Body)
                    .child(crate::dot(tokens.status().success.foreground).size(px(8.0)))
                    .when(!compact, |status| status.child("Available · 24 models")),
            )
    }
}

fn measure(
    width: f32,
    marked: bool,
    errors: bool,
    tokens: &Tokens,
    cx: &mut TestAppContext,
) -> Rc<RefCell<Measurements>> {
    let tokens = *tokens;
    let seen = Rc::new(RefCell::new(Measurements::empty()));
    let _window = cx.open_window(size(px(width), px(900.0)), {
        let seen = Rc::clone(&seen);
        move |_, _| Probe {
            width,
            marked,
            errors,
            tokens,
            seen,
        }
    });
    cx.run_until_parked();
    seen
}
#[gpui::test]
fn full_quiet_boxes_fit_first_following_last_and_error_rows_without_changing_names(
    cx: &mut TestAppContext,
) {
    for (width, inner) in [(1280.0, 686.0), (390.0, 356.0)] {
        for scheme in [crate::Scheme::Dark, crate::Scheme::Light] {
            for errors in [false, true] {
                let tokens = Tokens::of(scheme);
                let baseline = measure(width, false, errors, &tokens, cx);
                let marked = measure(width, true, errors, &tokens, cx);
                let baseline = baseline.borrow();
                let marked = marked.borrow();
                assert_eq!(marked.rows.len(), 6);
                assert_eq!(
                    marked.rows, baseline.rows,
                    "shared frame/pitch must not change"
                );
                for index in 0..6 {
                    let row = marked.rows[index];
                    if !errors || (index != 0 && index != 5) {
                        assert_eq!(row.size.height, px(56.0));
                    }
                    let slot = marked.slots[index][1];
                    assert_eq!(slot.size.width, px(32.0));
                    assert_eq!(slot.size.height, px(if index == 0 { 24.0 } else { 23.0 }));
                    let image = marked.images[index][0];
                    if index == 4 {
                        assert_eq!(image.size, size(px(16.0), px(16.0)));
                    }
                    if index == 0 || index == 5 {
                        assert_eq!(image.size, size(px(32.0), px(36.0)));
                        assert_eq!(
                            image.origin.y,
                            shift(row.origin.y, px(10.0)),
                            "no half-point OpenCode placement"
                        );
                        assert!(image.origin.y >= row.origin.y);
                        assert!(image.bottom() <= row.bottom());
                    }
                    let title = marked.titles[index][0];
                    let old_title = baseline.titles[index][0];
                    assert_eq!(
                        title.origin.y, old_title.origin.y,
                        "name baseline unchanged"
                    );
                    assert_eq!(
                        title.origin.x,
                        shift(old_title.origin.x, px(16.0)),
                        "only the documented common column changes"
                    );
                    let indicator = marked.titles[index].last().unwrap();
                    assert!(shift(title.right(), px(8.0)) <= indicator.origin.x);
                    assert!(indicator.right() <= shift(row.right(), px(-16.0)));
                    if let Some(previous) = index.checked_sub(1).and_then(|at| marked.rows.get(at))
                    {
                        assert!(previous.bottom() <= image.origin.y);
                    }
                    if let Some(next) = marked.rows.get(index.saturating_add(1)) {
                        assert!(image.bottom() <= next.origin.y);
                    }
                }
                assert_eq!(marked.rows[0].origin.y, px(17.0));
                assert_eq!(marked.rows[0].size.width, px(inner));
            }
        }
    }
}

fn shift(value: Pixels, by: Pixels) -> Pixels {
    px(f32::from(value).mul_add(1.0, f32::from(by)))
}
