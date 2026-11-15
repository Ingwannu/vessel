//! An agent's plan in the timeline: Paseo's task list
//! (`components/message.tsx`, `TodoListCard` in its `ExpandableBadge`;
//! `components/task-list-row.tsx`, `TaskListRow`). What each row says is
//! the view's (`view::timeline::PlanActivity`).
//!
//! [Decision Log]
//! - Purpose: a plan update was one quiet "Plan · 1 of 4 done" line, so
//!   a person could not tell what had changed or see the steps.
//! - The reference: Paseo's row has the activity's mark (a plus for a step
//!   added, the ring with a dot for one started, a check for one done, a
//!   checked square for a list created), its words in `foregroundMuted`,
//!   the step's own words after them, one step quieter and cut to a line;
//!   pressed, it opens to the plan's steps, each a 16-point mark (a ring
//!   to do, the ring with a dot in the running blue under way, a checked
//!   ring done) and its words (muted to do, the foreground under way,
//!   extra muted and struck through done; a step under way says its
//!   `activeForm` when it has one).
//! - Constraint: the timeline's rows are GOAT's work rows (a chevron, the
//!   label at 12.5, a rule-less row); the plan row is one of them, with
//!   Paseo's mark and words, so the lane reads as one.
//! - Differences, kept: the chevron stands before the mark on every row
//!   (Paseo shows it in the mark's place on hover); ACP's plan steps have
//!   no `activeForm`, so a step under way says its own words.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, FluentBuilder as _, InteractiveElement as _,
    IntoElement as _, ParentElement as _, StatefulInteractiveElement as _, Styled, div, px,
};
use vessel_app_kit::{Type, TypeExt as _};
use vessel_app_view::timeline::PlanActivity;
use vessel_proto::{PlanEntry, PlanStatus};

use super::entries::{Paint, disclosure, id};
use super::look::{WORK_LINE, WORK_SIZE};
use crate::shell::Shell;

/// A plan row: what changed, the step it names; open, the plan's steps.
pub(super) fn row(
    paint: &Paint<'_>,
    index: usize,
    (entries, activity): (&[PlanEntry], &PlanActivity),
    open: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let neutral = paint.tokens.neutral;
    let seat = paint.seat;
    let mark = match activity {
        PlanActivity::Created { .. } => Glyph::SquareCheck,
        PlanActivity::Added(_) => Glyph::Plus,
        PlanActivity::Started(_) => Glyph::CircleDot,
        PlanActivity::Completed(_) => Glyph::Check,
    };
    let ink = paint.look.secondary;
    let hover = neutral.foreground;
    let head = div()
        .id(ElementId::NamedInteger("plan".into(), id(index)))
        .flex()
        .items_center()
        .gap(px(6.0))
        .min_h(px(WORK_LINE))
        .px(px(4.0))
        .rounded(px(6.0))
        .face(Type::Meta)
        .text_size(px(vessel_app_kit::fonts::ui(WORK_SIZE)))
        .line_height(px(vessel_app_kit::fonts::ui(WORK_LINE)))
        .text_color(ink)
        .cursor_pointer()
        .hover(move |row| row.text_color(hover))
        .on_click(cx.listener(move |shell, _event, _window, cx| {
            shell.toggle_entry(seat, index, cx);
        }))
        .child(disclosure(open, ink))
        .child(glyph(mark, 14.0, ink))
        .child(div().flex_none().child(paint.words.plan_activity(activity)))
        .children(activity.step().map(|step| {
            div()
                .min_w(px(0.0))
                .truncate()
                .text_color(neutral.foreground_extra_muted)
                .child(step.to_owned())
        }));
    div()
        .child(head)
        .when(open, |row| row.child(steps(paint, entries)))
        .into_any_element()
}

/// The plan's steps, opened under its row (Paseo's details: 8 inside, 4
/// between).
fn steps(paint: &Paint<'_>, entries: &[PlanEntry]) -> Div {
    let list = div()
        .ml(px(28.0))
        .p(px(8.0))
        .flex()
        .flex_col()
        .gap(px(4.0))
        .face(Type::Body)
        .text_size(px(14.0));
    if entries.is_empty() {
        return list.child(
            div()
                .text_color(paint.tokens.neutral.foreground_muted)
                .child(paint.words.plan_empty()),
        );
    }
    list.children(entries.iter().map(|step| task(paint, step)))
}

/// One step: Paseo's `TaskListRow`.
fn task(paint: &Paint<'_>, step: &PlanEntry) -> Div {
    let neutral = paint.tokens.neutral;
    let (mark, mark_ink, text_ink, struck) = match step.status {
        PlanStatus::Completed => (
            Glyph::CircleCheck,
            neutral.foreground_extra_muted,
            neutral.foreground_extra_muted,
            true,
        ),
        PlanStatus::InProgress => (
            Glyph::CircleDot,
            paint.tokens.status().running_dot,
            neutral.foreground,
            false,
        ),
        PlanStatus::Pending => (
            Glyph::Circle,
            neutral.foreground_extra_muted,
            neutral.foreground_muted,
            false,
        ),
    };
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(div().flex_none().child(glyph(mark, 16.0, mark_ink)))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .truncate()
                .text_color(text_ink)
                .when(struck, Styled::line_through)
                .child(step.content.clone()),
        )
}
