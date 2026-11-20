//! The pane's checks section: Paseo's `pull-request-panel/checks-section.tsx`
//! and `checks-ring.tsx` (pinned 0.15.4). A pressable header (the ring at
//! 20, the headline, the detail line, a chevron) folds the list; the list
//! (at most 268 high, its own scroll) has a header per group that folds
//! it, and a 32-high row per check (its glyph, name, workflow, and its
//! time at the end) that opens the check on the forge.
//!
//! [Decision Log]
//! - Purpose: every check of the head, grouped by what to act on, as
//!   Paseo's pane lists them.
//! - Constraint: a node before `PULL_REQUEST_PANE_PROTOCOL` sends counts
//!   and the failing checks only (ADR 0088 part 2).
//! - Choice: the runs when the node sends them; otherwise the headline,
//!   the counts and the failing rows, as the facts dialog drew them. The
//!   ring is painted (GPUI draws no SVG built at run time), as the context
//!   meter's.
//! - A failed row's "Add to chat" reads the check whole first
//!   (`pane_attach`). Difference: the arcs end square (Paseo's are round).

use std::collections::HashSet;

use gpui::MouseButton;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::theme::SidebarSurfaces;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, FluentBuilder as _, Hsla, InteractiveElement as _, IntoElement as _,
    ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_view::workspace::pull::attach::can_add_check;
use vessel_app_view::workspace::pull::checks::{
    Presentation, RING_SIZE, RING_STROKE, Segment, Summary, circumference, segments, summary,
    timing,
};
use vessel_app_view::workspace::pull::{ChecksOutcome, check_counts};
use vessel_proto::forge::{CheckRun, ForgeUrl, PullRequest};

use super::Facts;
use super::pane_attach::check_key;
use crate::shell::Shell;

/// Paseo's list: roughly eight rows, then its own scroll.
const LIST_MAX: f32 = 268.0;

/// A presentation's colour (Paseo's ring palette and row glyphs).
pub(super) fn colour(presentation: Presentation, tokens: &Tokens) -> Hsla {
    let status = tokens.status();
    match presentation {
        Presentation::Failure => status.error.foreground,
        Presentation::Pending => status.warning.foreground,
        Presentation::Success => status.success.foreground,
        Presentation::Ignored => tokens.neutral.foreground_extra_muted,
    }
}

/// A row's glyph (Paseo's `CheckPresentationIcon`).
fn mark(presentation: Presentation, tokens: &Tokens) -> AnyElement {
    let (which, ink) = match presentation {
        Presentation::Failure => (Glyph::CircleX, colour(presentation, tokens)),
        Presentation::Pending => (Glyph::CircleDot, colour(presentation, tokens)),
        Presentation::Success => (Glyph::CircleCheck, colour(presentation, tokens)),
        Presentation::Ignored => (Glyph::CircleSlash, tokens.neutral.foreground_muted),
    };
    glyph(which, 14.0, ink).flex_none().into_any_element()
}

/// What the section shows: every check, or an older node's counts.
enum Shown<'a> {
    Every(Summary<'a>),
    Counts,
}

/// The section.
pub(super) fn section(
    shell: &Shell,
    facts: &Facts,
    pull: &PullRequest,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let checks = &pull.checks;
    let counted = checks
        .passed
        .saturating_add(checks.failed)
        .saturating_add(checks.pending);
    let shown = if checks.runs.is_empty() && counted > 0 {
        Shown::Counts
    } else {
        Shown::Every(summary(&checks.runs))
    };
    let open = !facts.choices.checks_closed;
    let (outcome, detail, parts, total) = match &shown {
        Shown::Every(read) => (
            read.outcome,
            read.detail_words(shell.words),
            read.parts(),
            read.total,
        ),
        Shown::Counts => {
            let counts: Vec<String> = check_counts(checks)
                .into_iter()
                .map(|count| count.words(shell.words))
                .collect();
            let parts = [
                (Presentation::Failure, checks.failed),
                (Presentation::Pending, checks.pending),
                (Presentation::Success, checks.passed),
            ]
            .into_iter()
            .filter(|&(_, count)| count > 0)
            .map(|(presentation, count)| (presentation, usize::try_from(count).unwrap_or(0)))
            .collect();
            (
                ChecksOutcome::of(checks.state),
                counts.join(", "),
                parts,
                usize::try_from(counted).unwrap_or(0),
            )
        }
    };
    let list: Option<AnyElement> = match &shown {
        Shown::Every(read) if open && !read.groups.is_empty() => Some(groups(
            shell,
            read,
            &facts.choices.collapsed,
            &pull.summary.url,
            tokens,
            cx,
        )),
        Shown::Counts if open => Some(failing(pull, tokens)),
        Shown::Every(_) | Shown::Counts => None,
    };
    div()
        .id("pr-pane-checks")
        .flex()
        .flex_col()
        .flex_none()
        .child(head(
            shell,
            outcome,
            detail,
            &segments(&parts, total),
            open,
            tokens,
            cx,
        ))
        .children(list)
        .into_any_element()
}

/// The section's header: the ring, the headline over the detail line, and
/// the chevron; a press folds the list.
fn head(
    shell: &Shell,
    outcome: ChecksOutcome,
    detail: String,
    arcs: &[Segment],
    open: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let neutral = tokens.neutral;
    let hover = SidebarSurfaces::of(tokens).hover;
    div()
        .id("pr-pane-checks-head")
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S2))
        .p(px(spacing::S3))
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .on_click(cx.listener(|shell, _event, _window, cx| {
            if let Some(facts) = shell.areas.pulls.facts.as_mut() {
                facts.choices.checks_closed = !facts.choices.checks_closed;
            }
            cx.notify();
        }))
        .child(super::pane_ring::ring(arcs, tokens))
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap(px(spacing::S0_5))
                .child(
                    div()
                        .type_(Type::Body)
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(neutral.foreground)
                        .truncate()
                        .child(outcome.words(shell.words).to_owned()),
                )
                .when(!detail.is_empty(), |text| {
                    text.child(
                        div()
                            .id("pr-pane-check-summary")
                            .type_(Type::Meta)
                            .text_color(neutral.foreground_muted)
                            .truncate()
                            .child(detail),
                    )
                }),
        )
        .child(
            glyph(
                if open {
                    Glyph::ChevronDown
                } else {
                    Glyph::ChevronRight
                },
                14.0,
                neutral.foreground_muted,
            )
            .flex_none(),
        )
        .into_any_element()
}

/// The groups, each with its header and, unless folded, its rows.
fn groups(
    shell: &Shell,
    read: &Summary<'_>,
    collapsed: &HashSet<Presentation>,
    fallback: &ForgeUrl,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let neutral = tokens.neutral;
    let mut list = div()
        .id("pr-pane-checks-list")
        .flex()
        .flex_col()
        .max_h(px(LIST_MAX))
        .overflow_y_scroll()
        .pb(px(spacing::S2));
    for group in &read.groups {
        let presentation = group.presentation;
        let folded = collapsed.contains(&presentation);
        list = list.child(
            div()
                .id(SharedString::from(format!(
                    "pr-pane-check-group-{presentation:?}"
                )))
                .flex()
                .flex_row()
                .items_center()
                .gap(px(spacing::S1))
                .px(px(spacing::S3))
                .pt(px(spacing::S2))
                .pb(px(spacing::S1))
                .cursor_pointer()
                .on_click(cx.listener(move |shell, _event, _window, cx| {
                    if let Some(facts) = shell.areas.pulls.facts.as_mut() {
                        let collapsed = &mut facts.choices.collapsed;
                        if !collapsed.remove(&presentation) {
                            let _new = collapsed.insert(presentation);
                        }
                    }
                    cx.notify();
                }))
                .child(
                    div()
                        .type_(Type::Meta)
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(neutral.foreground_muted)
                        .child(presentation.group_words(group.checks.len(), shell.words)),
                )
                .child(glyph(
                    if folded {
                        Glyph::ChevronRight
                    } else {
                        Glyph::ChevronDown
                    },
                    12.0,
                    neutral.foreground_muted,
                )),
        );
        if !folded {
            list = list.children(
                group
                    .checks
                    .iter()
                    .map(|run| row(shell, run, presentation, fallback, tokens, cx)),
            );
        }
    }
    list.into_any_element()
}

/// One check: its glyph, name and workflow, its time at the end; a press
/// opens it (else the pull request) on the forge.
fn row(
    shell: &Shell,
    run: &CheckRun,
    presentation: Presentation,
    fallback: &ForgeUrl,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let neutral = tokens.neutral;
    let hover = SidebarSurfaces::of(tokens).hover;
    let url = run.url.clone().unwrap_or_else(|| fallback.clone());
    let time = timing(run, chrono::Utc::now()).map(|timing| timing.words(shell.words));
    let key = check_key(run);
    let add = (shell.pulls_can_attach() && can_add_check(run)).then(|| {
        let adding = shell
            .areas
            .pulls
            .facts
            .as_ref()
            .is_some_and(|facts| facts.adding.checks.contains(&key));
        let run = run.clone();
        let words = shell.words.pulls();
        let label = if adding {
            words.adding_to_chat()
        } else {
            words.add_to_chat()
        };
        add_button(
            SharedString::from(format!("pr-pane-check-add-{key}")),
            label,
            adding,
            tokens,
        )
        .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
            cx.stop_propagation();
        })
        .when(!adding, |button| {
            button.on_click(cx.listener(move |shell, _event, window, cx| {
                cx.stop_propagation();
                shell.pulls_add_check(&run, window, cx);
            }))
        })
    });
    div()
        .id(SharedString::from(format!("pr-pane-check-row-{key}")))
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S2))
        .px(px(spacing::S3))
        .h(px(spacing::S8))
        .flex_none()
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .on_click(move |_event, _window, cx| cx.open_url(url.as_str()))
        .child(mark(presentation, tokens))
        .child(
            div()
                .type_(Type::Body)
                .text_color(neutral.foreground)
                .min_w_0()
                .flex_shrink_1()
                .truncate()
                .child(run.name.as_str().to_owned()),
        )
        .children(run.workflow.as_ref().map(|workflow| {
            div()
                .type_(Type::Meta)
                .text_color(neutral.foreground_muted)
                .min_w_0()
                .flex_shrink_1()
                .truncate()
                .child(workflow.as_str().to_owned())
        }))
        .child(
            div()
                .ml_auto()
                .flex()
                .flex_row()
                .flex_none()
                .items_center()
                .gap(px(spacing::S2))
                .children(add)
                .child(
                    div()
                        .type_(Type::Meta)
                        .text_color(neutral.foreground_muted)
                        .children(time),
                ),
        )
        .into_any_element()
}

/// Paseo's ghost "Add to chat" button (`MessageSquarePlus`, 4 across),
/// labelled `label`; a spinner for the mark while `adding`.
pub(super) fn add_button(
    id: SharedString,
    label: &str,
    adding: bool,
    tokens: &Tokens,
) -> gpui::Stateful<gpui::Div> {
    let neutral = tokens.neutral;
    let hover = SidebarSurfaces::of(tokens).hover;
    let icon: AnyElement = if adding {
        vessel_app_kit::slider::spinner(
            SharedString::from(format!("{id}-spinner")),
            12.0,
            neutral.foreground_muted,
        )
    } else {
        glyph(Glyph::MessagePlus, 12.0, neutral.foreground_muted).into_any_element()
    };
    div()
        .id(id)
        .flex()
        .flex_row()
        .flex_none()
        .items_center()
        .gap(px(spacing::S1))
        .h(px(spacing::S6))
        .px(px(spacing::S1))
        .rounded(px(spacing::S1))
        .type_(Type::Meta)
        .text_color(neutral.foreground)
        .when(!adding, |button| {
            button.cursor_pointer().hover(move |style| style.bg(hover))
        })
        .child(icon)
        .child(label.to_owned())
}

/// An older node's failing checks, as the facts dialog listed them.
fn failing(pull: &PullRequest, tokens: &Tokens) -> AnyElement {
    let neutral = tokens.neutral;
    div()
        .flex()
        .flex_col()
        .pb(px(spacing::S2))
        .children(pull.checks.failing.iter().map(|run| {
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(spacing::S2))
                .px(px(spacing::S3))
                .h(px(spacing::S8))
                .child(mark(Presentation::of(run.state), tokens))
                .child(
                    div()
                        .type_(Type::Body)
                        .text_color(neutral.foreground)
                        .min_w_0()
                        .truncate()
                        .child(run.name.as_str().to_owned()),
                )
        }))
        .into_any_element()
}
