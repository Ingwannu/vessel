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
    if let TimelineRead::Ready(timeline) = &facts.timeline {
        let Counts {
            approvals,
            changes,
            comments,
        } = counts(&shown(&timeline.items));
        let count_pill = |which, size, ink, count: usize| {
            (count > 0).then(|| pill(which, size, ink, &count.to_string()))
        };
        summary.extend(count_pill(
            Glyph::CircleCheck,
            12.0,
            status.success.foreground,
            approvals,
        ));
        summary.extend(count_pill(Glyph::CircleX, 12.0, status.error.foreground, changes));
        summary.extend(count_pill(
            Glyph::Message,
            11.0,
            neutral.foreground_muted,
            comments,
        ));
    } else if pills.approved {
        summary.push(pill(
            Glyph::CircleCheck,
            12.0,
            status.success.foreground,
            words.approved(),
        ));
    }
    if pills.changes_requested && !matches!(facts.timeline, TimelineRead::Ready(_)) {
        summary.push(pill(
            Glyph::CircleX,
            12.0,
            status.error.foreground,
            words.changes_requested(),
        ));
    }
    if pills.unresolved > 0 && !matches!(facts.timeline, TimelineRead::Ready(_)) {
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
        let feedback_row = super::feedback::row(
            shell,
            feedback,
            tokens,
            breakpoint::phone(window),
            cx,
        );
        let attach = shell.pulls_can_attach();
        let cards = Cards {
            shell,
            choices: &facts.choices,
            tokens,
            now: chrono::Utc::now(),
            attach,
        };
        let listed: Option<Vec<AnyElement>> = match &facts.timeline {
            TimelineRead::Ready(timeline) => {
                let items = shown(&timeline.items);
                let made = entries(&items);
                Some(if made.is_empty() {
                    vec![no_activity(shell, tokens)]
                } else {
                    // Paseo's activity toolbar: "Add all to chat".
                    let all = attach.then(|| {
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .min_h(px(28.0))
                            .px(px(8.0))
                            .pb(px(8.0))
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
                    all.into_iter()
                        .chain(made.iter().map(|entry| cards.entry(entry, cx)))
                        .collect()
                })
            }
            TimelineRead::Loading => Some(vec![super::pane_skeleton::activity(tokens)]),
            TimelineRead::Failed => Some(vec![no_activity(shell, tokens)]),
            TimelineRead::Unasked => None,
        };
        if let Some(listed) = listed {
            section = section.child(
                div()
                    .flex()
                    .flex_col()
                    .pb(px(12.0))
                    .children(listed)
                    .child(div().px(px(12.0)).child(feedback_row)),
            );
            return section.into_any_element();
        }
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
                .child(feedback_row),
        );
    }
    section.into_any_element()
}

/// Paseo's empty Activity.
fn no_activity(shell: &Shell, tokens: &Tokens) -> AnyElement {
    div()
        .px(px(12.0))
        .py(px(8.0))
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
        .gap(px(3.0))
        .text_color(ink)
        .child(glyph(which, size, ink).flex_none())
        .child(text.to_owned())
        .into_any_element()
}
