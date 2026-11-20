//! The status marks against Paseo's geometry and in GPUI's real
//! accessibility tree: a badge and a workspace's slot say their bucket as
//! a status, a done project has no badge, every status draws in a slot.
use super::*;
use crate::a11y_snapshot::{activate, nodes};
use crate::{Scheme, Tokens};
use gpui::{Render, TestAppContext, VisualTestContext, Window, div, px};

const MARKS: [StatusMark; 5] = [
    StatusMark::NeedsInput,
    StatusMark::Failed,
    StatusMark::Running,
    StatusMark::Attention,
    StatusMark::Done,
];

fn words(mark: StatusMark) -> &'static str {
    match mark {
        StatusMark::NeedsInput => "Needs input",
        StatusMark::Failed => "Failed",
        StatusMark::Running => "Working",
        StatusMark::Attention => "Ready to review",
        StatusMark::Done => "Done",
    }
}

struct Probe;

impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let knockout = Knockout {
            group: "row".into(),
            selected: false,
        };
        let mut column = div().id("row").group("row").flex().flex_col().gap(px(8.0));
        for (index, mark) in MARKS.into_iter().enumerate() {
            let tile = div().size(px(16.0)).bg(tokens.neutral.surface2);
            column = column
                .child(status_slot(
                    ("slot", index),
                    mark,
                    words(mark),
                    Glyph::FolderGit,
                    false,
                    &tokens,
                ))
                .child(marked(
                    tile.into_any_element(),
                    false,
                    status_badge(("badge", index), mark, words(mark), &knockout, &tokens),
                ));
        }
        column
    }
}

#[test]
fn geometry_is_paseos_budget() {
    // The ring: the 6 dot, then a 1.5 gap and a 1.5 stroke each side.
    assert_eq!(4.0_f32.mul_add(1.5, FILLED_DOT).to_bits(), RING.to_bits());
    // Its frame: one more each side, the knockout margin.
    assert_eq!(
        2.0_f32.mul_add(RING_INSET, RING).to_bits(),
        RING_FRAME.to_bits()
    );
    // `getStatusRingOffset(-4, 12)`: the shell's corner offset, less half
    // what the frame outgrows the shell by.
    let offset = (-0.5_f32).mul_add(RING_FRAME, 0.5_f32.mul_add(BADGE, BADGE_OFFSET));
    assert_eq!(offset.to_bits(), RING_OFFSET.to_bits());
    // The density icon box ("md"): 16 wide, 20 on a phone.
    assert_eq!(icon_box(false).to_bits(), 16.0_f32.to_bits());
    assert_eq!(icon_box(true).to_bits(), 20.0_f32.to_bits());
    assert_eq!(glyph_box(false).to_bits(), 14.0_f32.to_bits());
    assert_eq!(glyph_box(true).to_bits(), 18.0_f32.to_bits());
    assert_eq!(PERIOD, Duration::from_millis(900));
}

#[test]
fn a_done_project_has_no_badge_and_every_other_status_has_one() {
    let tokens = Tokens::of(Scheme::Light);
    let knockout = Knockout {
        group: "row".into(),
        selected: true,
    };
    for mark in MARKS {
        let badge = status_badge("badge", mark, words(mark), &knockout, &tokens);
        assert_eq!(badge.is_none(), mark == StatusMark::Done, "{mark:?}");
    }
}

#[gpui::test]
fn each_badge_and_each_slot_says_its_bucket_as_a_status(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init(cx, crate::SchemeChoice::Fixed(Scheme::Dark)).unwrap();
        cx.set_reduce_motion(true);
    });
    let (view, cx): (_, &mut VisualTestContext) = cx.add_window_view(|_, _| Probe);
    cx.run_until_parked();
    activate(cx);
    view.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
    let mut said: Vec<Option<String>> = cx
        .update(|window, _| nodes(window, "Status").unwrap())
        .into_iter()
        .map(|node| node.label)
        .collect();
    said.sort();
    // Every slot (done included, TRACKER 813) and every badge but done's.
    let expected: Vec<Option<String>> = [
        "Done",
        "Failed",
        "Failed",
        "Needs input",
        "Needs input",
        "Ready to review",
        "Ready to review",
        "Working",
        "Working",
    ]
    .into_iter()
    .map(|label| Some(label.to_owned()))
    .collect();
    assert_eq!(said, expected);
}
