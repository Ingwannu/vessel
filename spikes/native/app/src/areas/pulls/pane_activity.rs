//! The pane's Activity section: Paseo's `Section` (`section-kit.tsx`), a
//! pressable header (the chevron, "Activity", its pills at the end) that
//! folds the body: the timeline's cards (`pane_cards`), a skeleton while
//! the first read is out, or "No activity yet".
//!
//! [Decision Log]
//! - Purpose: what reviewers said, where Paseo lists it.
//! - Constraint: a node before `PULL_REQUEST_PANE_PROTOCOL` has no
//!   timeline (Paseo then shows "No activity yet").
//! - Choice: with a timeline, Paseo's pills (approvals, change requests,
//!   comments) and cards; without one, the node's review standing
//!   (approved, changes requested, unresolved threads, the verdict and
//!   whether it can merge), which says more than nothing. The feedback
//!   switch (Vessel's: failing checks and review comments sent to the
//!   session) closes the section either way.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::theme::SidebarSurfaces;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Hsla, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, breakpoint};
use vessel_app_view::workspace::pull::timeline::{Counts, counts, entries, shown};
use vessel_app_view::workspace::pull::{Merge, ReviewPills};
use vessel_proto::forge::PullRequest;

use super::Facts;
use super::pane_cards::Cards;
use super::pane_timeline::TimelineRead;
use crate::shell::Shell;

/// A summary pill's gap between its glyph and count (Paseo's 3).
const PILL_GAP: f32 = 3.0;

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
    let open = !facts.choices.activity_closed;
    let summary = summary_pills(shell, facts, pull, tokens);
    let mut section = div()
        .id("pr-pane-activity")
        .flex()
        .flex_col()
        .flex_none()
        .child(head(shell, open, summary, tokens, cx));
    if open {
        let feedback_row =
            super::feedback::row(shell, feedback, tokens, breakpoint::phone(window), cx);
        let body = match timeline_body(shell, facts, tokens, cx) {
            Some(listed) => div()
                .flex()
                .flex_col()
                .pb(px(spacing::S3))
                .children(listed)
                .child(div().px(px(spacing::S3)).child(feedback_row)),
            None => div()
                .flex()
                .flex_col()
                .gap(px(spacing::S2))
                .px(px(spacing::S3))
                .pb(px(spacing::S3))
                .children(standing(shell, pull, tokens))
                .child(feedback_row),
        };
        section = section.child(body);
    }
    section.into_any_element()
}

/// The header's pills: Paseo's counts of the timeline, or the node's
/// standing without one.
fn summary_pills(
    shell: &Shell,
    facts: &Facts,
    pull: &PullRequest,
    tokens: &Tokens,
) -> Vec<AnyElement> {
    let words = shell.words.pulls();
    let muted = tokens.neutral.foreground_muted;
    let status = tokens.status();
    if let TimelineRead::Ready(timeline) = &facts.timeline {
        let Counts {
            approvals,
            changes,
            comments,
        } = counts(&shown(&timeline.items));
        return [
            (
                Glyph::CircleCheck,
                12.0,
                status.success.foreground,
                approvals,
            ),
            (Glyph::CircleX, 12.0, status.error.foreground, changes),
            (Glyph::Message, 11.0, muted, comments),
        ]
        .into_iter()
        .filter(|&(_, _, _, count)| count > 0)
        .map(|(which, size, ink, count)| pill(which, size, ink, &count.to_string()))
        .collect();
    }
    let pills = ReviewPills::of(&pull.reviews);
    let mut summary = Vec::new();
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
        summary.push(pill(Glyph::Message, 11.0, muted, &threads));
    }
    summary
}

/// Paseo's section header: the chevron, "Activity", the pills at the end;
/// a press folds the body.
fn head(
    shell: &Shell,
    open: bool,
    summary: Vec<AnyElement>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let neutral = tokens.neutral;
    let hover = SidebarSurfaces::of(tokens).hover;
    div()
        .id("pr-pane-activity-head")
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S2))
        .px(px(spacing::S3))
        .py(px(spacing::S2))
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
                .child(shell.words.pulls().activity().to_owned()),
        )
        .child(
            div()
                .ml_auto()
                .flex()
                .flex_row()
                .flex_wrap()
                .items_center()
                .gap(px(spacing::S2))
                .type_(Type::Meta)
                .children(summary),
        )
        .into_any_element()
}

/// The timeline's body: "Add all to chat" and the cards, the skeleton,
/// or "No activity yet"; `None` for a node without a timeline.
fn timeline_body(
    shell: &Shell,
    facts: &Facts,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<Vec<AnyElement>> {
    let timeline = match &facts.timeline {
        TimelineRead::Ready(timeline) => timeline,
        TimelineRead::Loading => return Some(vec![super::pane_skeleton::activity(tokens)]),
        TimelineRead::Failed => return Some(vec![no_activity(shell, tokens)]),
        TimelineRead::Unasked => return None,
    };
    let items = shown(&timeline.items);
    let made = entries(&items);
    if made.is_empty() {
        return Some(vec![no_activity(shell, tokens)]);
    }
    let attach = shell.pulls_can_attach();
    let cards = Cards {
        shell,
        choices: &facts.choices,
        tokens,
        now: chrono::Utc::now(),
        attach,
    };
    // Paseo's activity toolbar: "Add all to chat".
    let all = attach.then(|| {
        div()
            .flex()
            .flex_row()
            .items_center()
            .min_h(px(spacing::S7))
            .px(px(spacing::S2))
            .pb(px(spacing::S2))
            .child(
                super::pane_checks::add_button(
                    "pr-pane-add-all".into(),
                    shell.words.pulls().add_all_to_chat(),
                    false,
                    tokens,
                )
                .on_click(cx.listener(|shell, _event, window, cx| {
                    shell.pulls_add_all(window, cx);
                })),
            )
            .into_any_element()
    });
    Some(
        all.into_iter()
            .chain(made.iter().map(|entry| cards.entry(entry, cx)))
            .collect(),
    )
}

/// The node's review standing: the verdict and whether it can merge.
fn standing(shell: &Shell, pull: &PullRequest, tokens: &Tokens) -> Option<AnyElement> {
    let verdict = ReviewPills::of(&pull.reviews)
        .remaining_verdict(pull.summary.review)
        .words(shell.words);
    let merge = Merge::for_open(&pull.summary).map(|merge| merge.words(shell.words));
    let text = match (verdict, merge) {
        (Some(verdict), Some(merge)) => format!("{verdict} · {merge}"),
        (Some(verdict), None) => verdict.to_owned(),
        (None, Some(merge)) => merge.to_owned(),
        (None, None) => return None,
    };
    Some(
        div()
            .type_(Type::Meta)
            .text_color(tokens.neutral.foreground)
            .child(text)
            .into_any_element(),
    )
}

/// Paseo's empty Activity.
fn no_activity(shell: &Shell, tokens: &Tokens) -> AnyElement {
    div()
        .px(px(spacing::S3))
        .py(px(spacing::S2))
        .type_(Type::Meta)
        .text_color(tokens.neutral.foreground_muted)
        .child(shell.words.pulls().no_activity().to_owned())
        .into_any_element()
}

/// One summary pill: its glyph and its words in the glyph's colour.
fn pill(which: Glyph, size: f32, ink: Hsla, text: &str) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(PILL_GAP))
        .text_color(ink)
        .child(glyph(which, size, ink).flex_none())
        .child(text.to_owned())
        .into_any_element()
}
