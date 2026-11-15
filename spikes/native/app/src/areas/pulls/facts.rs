//! A pull request's facts: Paseo's pull request pane
//! (`git/pull-request-panel/pane.tsx`): View and refresh at the top; the
//! title with its number, then its state and repository; the checks'
//! headline with their counts and the failing ones; the reviews' pills.
//!
//! Differences, each for a reason: the node sends counts and only the
//! failing checks (at most twenty), so Paseo's groups of passed and
//! skipped checks are counts, not lists; the node sends the reviews'
//! standing (approved, changes requested, unresolved threads), not the
//! activity timeline, so the Activity section is its summary pills and
//! the verdict; Paseo's "Add to chat" waits for the feedback slice
//! (`pr.feedback`, failing checks sent to the session).

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::dialog::{DialogContent, DialogSize};
use vessel_app_kit::picker::glyph;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, HighlightStyle, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, StyledText, Window, div, px,
};
use vessel_app_kit::{
    ButtonKind, ButtonState, Tokens, Type, TypeExt as _, breakpoint, button_with_kind,
};
use vessel_app_view::workspace::git::Issue;
use vessel_app_view::workspace::pull::{
    ChecksOutcome, Merge, PullPhase, ReviewPills, Verdict, check_counts,
};
use vessel_proto::forge::{CheckRun, CheckState, PullRequest, PullRequestView};

use super::{FactsRead, link::state_glyph};
use crate::shell::Shell;

/// Paseo's pill glyphs at 12.
const PILL_GLYPH: f32 = 12.0;

/// The divider between the facts' groups: one point, the kit's border line.
const DIVIDER: f32 = 1.0;
/// The checks' headline glyph (Paseo's ring at 16).
const HEADLINE_GLYPH: f32 = 16.0;

/// The facts, when open.
pub(crate) fn dialog(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let pulls = &shell.areas.pulls;
    let facts = pulls.facts.as_ref()?;
    let words = shell.words.pulls();
    let narrow = breakpoint::phone(window);
    let (body, url) = match &facts.read {
        FactsRead::Loading => (loading(tokens), None),
        FactsRead::Failed(problem) => (failed(shell, problem, tokens), None),
        FactsRead::Ready(view) => match &view.pull_request {
            Some(pull) => (
                whole(shell, pull, view.feedback, tokens, narrow, cx),
                Some(pull.summary.url.clone()),
            ),
            None => (empty(shell, view, tokens), None),
        },
    };
    let mut footer = vec![refresh(shell, facts.refreshing, tokens, cx)];
    if let Some(url) = url {
        footer.push(
            button_with_kind(
                "pulls-view",
                words.view(),
                ButtonKind::Secondary,
                ButtonState::Enabled,
                tokens,
            )
            .on_click(move |_event, _window, cx| cx.open_url(url.as_str()))
            .into_any_element(),
        );
    }
    Some(vessel_app_kit::dialog::dialog(
        &pulls.dialog,
        DialogContent {
            title: words.label().into(),
            body,
            footer,
            size: DialogSize::Normal,
        },
        tokens,
        window,
        narrow,
        Shell::pulls_close,
        cx,
    ))
}

/// Paseo's refresh (`refreshState`): read it from the forge now.
fn refresh(
    shell: &Shell,
    refreshing: bool,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let state = if refreshing {
        ButtonState::Disabled
    } else {
        ButtonState::Enabled
    };
    let button = button_with_kind(
        "pulls-refresh",
        shell.words.pulls().refresh(),
        ButtonKind::Ghost,
        state,
        tokens,
    );
    if refreshing {
        button.into_any_element()
    } else {
        button
            .on_click(cx.listener(|shell, _event, _window, cx| shell.pulls_refresh(cx)))
            .into_any_element()
    }
}

fn loading(tokens: &Tokens) -> AnyElement {
    div()
        .flex()
        .justify_center()
        .py(px(spacing::S6))
        .child(vessel_app_kit::slider::spinner(
            "pulls-loading",
            16.0,
            tokens.neutral.foreground_muted,
        ))
        .into_any_element()
}

fn failed(
    shell: &Shell,
    problem: &vessel_app_view::workspace::problem::Problem,
    tokens: &Tokens,
) -> AnyElement {
    let text = shell.t.lang.git_issue(&Issue::Call(problem.clone()));
    vessel_app_kit::alert::alert(
        vessel_app_kit::alert::Content {
            title: Some(text.title.clone().into()),
            description: Some(crate::areas::git::failure_body(&text, tokens)),
            action: None,
        },
        vessel_app_kit::alert::Variant::Error,
        vessel_app_kit::alert::Size::Small,
        tokens,
    )
    .into_any_element()
}

/// None read: Paseo's empty pane, or why the node cannot see it.
fn empty(shell: &Shell, view: &PullRequestView, tokens: &Tokens) -> AnyElement {
    let words = shell.words.pulls();
    let why = if matches!(view.access, vessel_proto::forge::ForgeAccess::Ready { .. }) {
        words.empty_description().to_owned()
    } else {
        vessel_app_view::workspace::pull::Unavailable::of(&view.access)
            .words(shell.words)
            .into_owned()
    };
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S1))
        .py(px(spacing::S4))
        .child(
            div()
                .type_(Type::Body)
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(tokens.neutral.foreground)
                .child(words.empty_title().to_owned()),
        )
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground_muted)
                .child(why),
        )
        .into_any_element()
}

/// The header, the checks and the reviews.
fn whole(
    shell: &Shell,
    pull: &PullRequest,
    feedback: bool,
    tokens: &Tokens,
    touch: bool,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S4))
        .child(header(shell, pull, tokens))
        .child(divider(tokens))
        .child(checks(shell, pull, tokens))
        .child(divider(tokens))
        .child(activity(shell, pull, tokens))
        .child(divider(tokens))
        .child(super::feedback::row(shell, feedback, tokens, touch, cx))
        .into_any_element()
}

fn divider(tokens: &Tokens) -> AnyElement {
    div()
        .h(px(DIVIDER))
        .w_full()
        .bg(tokens.neutral.border)
        .into_any_element()
}

/// Paseo's header (`pull-request-panel/pane.tsx` 524-552): the title at
/// the body size and weight with " #42" muted in the same run of text,
/// so a wrapped title carries the number at its end, then the state's
/// glyph and word, then the repository.
fn header(shell: &Shell, pull: &PullRequest, tokens: &Tokens) -> AnyElement {
    let neutral = tokens.neutral;
    let summary = &pull.summary;
    let phase = PullPhase::of(summary);
    let repo = summary.repo.path.as_str().to_owned();
    let mut title = summary.title.as_str().to_owned();
    let start = title.len();
    title.push_str(" #");
    title.push_str(&summary.number.get().to_string());
    let number = vec![(
        start..title.len(),
        HighlightStyle {
            color: Some(neutral.foreground_muted),
            ..HighlightStyle::default()
        },
    )];
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S1))
        .child(
            div()
                .type_(Type::Body)
                .text_color(neutral.foreground)
                .whitespace_normal()
                .child(StyledText::new(title).with_highlights(number)),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .items_center()
                .gap(px(spacing::S2))
                .type_(Type::Meta)
                .child(state_glyph(phase, tokens))
                .child(
                    div()
                        .text_color(neutral.foreground)
                        .child(phase.words(shell.words).to_owned()),
                )
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_color(neutral.foreground_muted)
                        .child(repo),
                ),
        )
        .into_any_element()
}

/// A section's small title.
fn section_title(text: &str, tokens: &Tokens) -> AnyElement {
    div()
        .type_(Type::Meta)
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(tokens.neutral.foreground_muted)
        .child(text.to_owned())
        .into_any_element()
}

/// Paseo's checks summary: the headline with its glyph, the counts, then
/// the failing ones by name.
fn checks(shell: &Shell, pull: &PullRequest, tokens: &Tokens) -> AnyElement {
    let words = shell.words.pulls();
    let outcome = ChecksOutcome::of(pull.checks.state);
    let status = tokens.status();
    let (which, colour) = match outcome {
        ChecksOutcome::Failure => (Glyph::CircleX, status.error.foreground),
        ChecksOutcome::Pending => (Glyph::CircleDot, status.warning.foreground),
        ChecksOutcome::Success => (Glyph::CircleCheck, status.success.foreground),
        ChecksOutcome::None | ChecksOutcome::Unknown => {
            (Glyph::Circle, tokens.neutral.foreground_muted)
        }
    };
    let counts: Vec<String> = check_counts(&pull.checks)
        .into_iter()
        .map(|count| count.words(shell.words))
        .collect();
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .child(section_title(words.checks(), tokens))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(spacing::S2))
                .child(glyph(which, HEADLINE_GLYPH, colour).flex_none())
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .min_w_0()
                        .child(
                            div()
                                .type_(Type::Body)
                                .text_color(tokens.neutral.foreground)
                                .child(outcome.words(shell.words).to_owned()),
                        )
                        .children((!counts.is_empty()).then(|| {
                            div()
                                .type_(Type::Meta)
                                .text_color(tokens.neutral.foreground_muted)
                                .child(counts.join(", "))
                        })),
                ),
        )
        .children(
            pull.checks
                .failing
                .iter()
                .map(|run| failing_row(run, tokens)),
        )
        .children(pull.checks.truncated.then(|| {
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .child(words.checks_more().to_owned())
        }))
        .into_any_element()
}

/// One failing check: its glyph and name, one line.
fn failing_row(run: &CheckRun, tokens: &Tokens) -> AnyElement {
    let status = tokens.status();
    let colour = match run.state {
        CheckState::Failed | CheckState::Cancelled => status.error.foreground,
        CheckState::Pending | CheckState::Passed | CheckState::Skipped | CheckState::Unknown => {
            tokens.neutral.foreground_muted
        }
    };
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S2))
        .pl(px(spacing::S6))
        .type_(Type::Meta)
        .text_color(tokens.neutral.foreground)
        .child(glyph(Glyph::CircleX, PILL_GLYPH, colour).flex_none())
        .child(
            div()
                .min_w_0()
                .truncate()
                .child(run.name.as_str().to_owned()),
        )
        .into_any_element()
}

/// Paseo's Activity summary: approvals and change requests as pills, the
/// unresolved threads, the verdict and whether it can merge.
fn activity(shell: &Shell, pull: &PullRequest, tokens: &Tokens) -> AnyElement {
    let words = shell.words.pulls();
    let status = tokens.status();
    let pills = ReviewPills::of(&pull.reviews);
    let mut row = div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_center()
        .gap(px(spacing::S3))
        .type_(Type::Meta)
        .text_color(tokens.neutral.foreground_muted);
    if pills.approved {
        row = row.child(pill(
            Glyph::CircleCheck,
            status.success.foreground,
            words.approved(),
            tokens,
        ));
    }
    if pills.changes_requested {
        row = row.child(pill(
            Glyph::CircleX,
            status.error.foreground,
            words.changes_requested(),
            tokens,
        ));
    }
    if pills.unresolved > 0 {
        let threads = words
            .unresolved(vessel_app_view::localization::Number::count(
                pills.unresolved,
            ))
            .unwrap_or_default();
        row = row.child(pill(
            Glyph::Message,
            tokens.neutral.foreground_muted,
            &threads,
            tokens,
        ));
    }
    let verdict = Verdict::of(pull.summary.review).words(shell.words);
    let merge = Merge::of(pull.summary.mergeable).words(shell.words);
    div()
        .flex()
        .flex_col()
        .gap(px(spacing::S2))
        .child(section_title(words.activity(), tokens))
        .child(row)
        .child(
            div()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground)
                .child(match verdict {
                    Some(verdict) => format!("{verdict} · {merge}"),
                    None => merge.to_owned(),
                }),
        )
        .into_any_element()
}

fn pill(which: Glyph, colour: gpui::Hsla, text: &str, tokens: &Tokens) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S1))
        .child(glyph(which, PILL_GLYPH, colour).flex_none())
        .child(
            div()
                .text_color(tokens.neutral.foreground)
                .child(text.to_owned()),
        )
        .into_any_element()
}
