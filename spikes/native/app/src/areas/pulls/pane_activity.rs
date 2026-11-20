//! The pane's Activity section: Paseo's `Section` (`section-kit.tsx`), a
//! pressable header (the chevron, "Activity", its pills at the end) that
//! folds the body.
//!
//! [Decision Log]
//! - Purpose: the reviews' standing beside the checks, where Paseo lists
//!   the activity.
//! - Constraint: the timeline (`pr.timeline`) is packet P3b; until then
//!   the node's review standing is what this section knows.
//! - Choice: the pills from the standing (approved, changes requested,
//!   unresolved threads), the verdict and whether it can merge, and the
//!   feedback switch (Vessel's: failing checks and review comments sent
//!   to the session), as the facts dialog had them.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::theme::SidebarSurfaces;
use vessel_app_kit::ui::{
    AnyElement, Context, Hsla, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, breakpoint};
use vessel_app_view::workspace::pull::{Merge, ReviewPills};
use vessel_proto::forge::PullRequest;

use super::Facts;
use crate::shell::Shell;

/// The section.
pub(super) fn section(
    shell: &Shell,
    facts: &Facts,
    pull: &PullRequest,
    feedback: bool,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.pulls();
    let neutral = tokens.neutral;
    let status = tokens.status();
    let open = !facts.choices.activity_closed;
    let pills = ReviewPills::of(&pull.reviews);
    let mut summary: Vec<AnyElement> = Vec::new();
    if pills.approved {
        summary.push(pill(
            Glyph::CircleCheck,
            12.0,
            status.success.foreground,
            words.approved(),
        ));
    }
    if pills.changes_requested {
        summary.push(pill(
            Glyph::CircleX,
            12.0,
            status.error.foreground,
            words.changes_requested(),
        ));
    }
    if pills.unresolved > 0 {
        let threads = words
            .unresolved(vessel_app_view::localization::Number::count(
                pills.unresolved,
            ))
            .unwrap_or_default();
        summary.push(pill(Glyph::Message, 11.0, neutral.foreground_muted, &threads));
    }
    let hover = SidebarSurfaces::of(tokens).hover;
    let head = div()
        .id("pr-pane-activity-head")
        .flex()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .px(px(12.0))
        .py(px(8.0))
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .on_click(cx.listener(|shell, _event, _window, cx| {
            if let Some(facts) = shell.areas.pulls.facts.as_mut() {
                facts.choices.activity_closed = !facts.choices.activity_closed;
            }
            cx.notify();
        }))
        .child(glyph(
            if open {
                Glyph::ChevronDown
            } else {
                Glyph::ChevronRight
            },
            14.0,
            neutral.foreground_muted,
        ))
        .child(
            div()
                .type_(Type::Meta)
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(neutral.foreground_muted)
                .child(words.activity().to_owned()),
        )
        .child(
            div()
                .ml_auto()
                .flex()
                .flex_row()
                .flex_wrap()
                .items_center()
                .gap(px(8.0))
                .type_(Type::Meta)
                .children(summary),
        );
    let mut section = div()
        .id("pr-pane-activity")
        .flex()
        .flex_col()
        .flex_none()
        .child(head);
    if open {
        let verdict = pills
            .remaining_verdict(pull.summary.review)
            .words(shell.words);
        let merge = Merge::for_open(&pull.summary).map(|merge| merge.words(shell.words));
        let standing = match (verdict, merge) {
            (Some(verdict), Some(merge)) => Some(format!("{verdict} · {merge}")),
            (Some(verdict), None) => Some(verdict.to_owned()),
            (None, Some(merge)) => Some(merge.to_owned()),
            (None, None) => None,
        };
        section = section.child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .px(px(12.0))
                .pb(px(12.0))
                .children(standing.map(|text| {
                    div()
                        .type_(Type::Meta)
                        .text_color(neutral.foreground)
                        .child(text)
                }))
                .child(super::feedback::row(
                    shell,
                    feedback,
                    tokens,
                    breakpoint::phone(window),
                    cx,
                )),
        );
    }
    section.into_any_element()
}

/// One summary pill: its glyph and its words in the glyph's colour.
fn pill(which: Glyph, size: f32, ink: Hsla, text: &str) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(3.0))
        .text_color(ink)
        .child(glyph(which, size, ink).flex_none())
        .child(text.to_owned())
        .into_any_element()
}
