//! The pull request pane, the code panel's Pull request tab: Paseo's
//! `git/pull-request-panel/pane.tsx` and `panels/pull-request/index.tsx`
//! (pinned 0.15.4). One scroll of the toolbar (View, and the refresh
//! icon), the header (the title with its number, the state, the
//! repository; a press opens it), the checks section (`pane_checks`), a
//! divider and the activity (`pane_activity`); a skeleton while it is
//! read, Paseo's error with Retry, and its empty words.
//!
//! [Decision Log]
//! - Purpose: the facts a person reads beside the session, as Paseo's
//!   pane, where the dialog covered the Changes tab.
//! - Constraint: the whole pull request is `pr.get`, read for the session
//!   the panel shows; the card's summary is kept fresh by the node.
//! - Choice: the read is kept with its session and the card's summary it
//!   answered; the tab, the card changing while the tab is shown, and the
//!   refresh icon read it again (Paseo's pane is refreshed when its status
//!   changes, never polled). A failed refresh keeps what was read.
//! - Differences: Paseo's error never says why (always "Failed to refresh
//!   git state."), kept; the View button is the kit's ghost at Paseo's 24.

use vessel_app_kit::theme::spacing;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::theme::SidebarSurfaces;
use vessel_app_kit::ui::{
    AnyElement, Context, FluentBuilder as _, HighlightStyle, InteractiveElement as _,
    IntoElement as _, ParentElement as _, StatefulInteractiveElement as _, Styled as _, StyledText,
    Window, div, px, relative,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_link::workspace::{Ask, PullAsk};
use vessel_app_view::workspace::pull::PullPhase;
use vessel_proto::forge::{GetPullRequest, PullRequest, PullRequestView, PullSummary};

use super::link::state_glyph_sized;
use super::{Facts, FactsRead, Subject};
use crate::shell::Shell;

/// The divider between the checks and the activity (Paseo's, one point).
const DIVIDER: f32 = 1.0;
/// The title's line height (Paseo's `title`, 22 on 14).
const TITLE_LINE: f32 = 22.0;

/// Paseo's `WORKSPACE_SECONDARY_HEADER_HEIGHT`.
const TOOLBAR: f32 = 36.0;
/// The header's state glyph.
const STATE_GLYPH: f32 = 14.0;

impl Shell {
    /// The card's summary of `session`'s pull request, when it names one.
    fn pulls_card(&self, machine: &str, session: vessel_proto::SessionId) -> Option<&PullSummary> {
        self.fleet
            .machines
            .iter()
            .find(|known| known.name == machine)?
            .sessions
            .iter()
            .find(|card| card.session == session)?
            .detail
            .pull_request
            .as_deref()?
            .summary
            .as_ref()
    }

    /// The bare number the Pull request tab is labelled with (Paseo's
    /// `formatPrTabLabel`), when the open session's card names one.
    pub(crate) fn pulls_tab_number(&self) -> Option<u64> {
        let (machine, session, _) = self.code_checkout()?;
        self.pulls_card(machine, session)
            .map(|summary| summary.number.get())
    }

    /// Read the pane's pull request when the tab shows a session not read,
    /// or whose card changed since (its checks moved, a review came).
    pub(crate) fn pulls_pane_read(&mut self) {
        let Some((machine, session, root)) = self
            .code_checkout()
            .map(|(machine, session, root)| (machine.to_owned(), session, *root))
        else {
            return;
        };
        let card = self.pulls_card(&machine, session).cloned();
        let pulls = &mut self.areas.pulls;
        let same = pulls.facts.as_ref().is_some_and(|facts| {
            facts.subject.machine == machine && facts.subject.session == session
        });
        if same {
            let Some(facts) = pulls.facts.as_mut() else {
                return;
            };
            let current = match &facts.read {
                FactsRead::Loading => true,
                FactsRead::Ready(_) => facts.seen == card || facts.refreshing,
                FactsRead::Failed(_) => false,
            };
            if current {
                return;
            }
            facts.seen.clone_from(&card);
            if matches!(facts.read, FactsRead::Failed(_)) {
                facts.read = FactsRead::Loading;
            } else {
                facts.refreshing = true;
            }
        } else {
            pulls.facts = Some(Facts {
                subject: Subject {
                    machine: machine.clone(),
                    session,
                    root,
                },
                read: FactsRead::Loading,
                refreshing: false,
                seen: card,
                timeline: super::pane_timeline::TimelineRead::default(),
                adding: super::pane_attach::Adding::default(),
                choices: super::Choices::default(),
            });
        }
        self.ask(
            &machine,
            Ask::Pull(PullAsk::Get(GetPullRequest {
                session,
                refresh: false,
            })),
        );
    }

    /// Machines and sessions changed: the shown pane follows its card.
    pub(crate) fn pulls_fleet(&mut self) {
        if self.code_shows_pull_request() {
            self.pulls_pane_read();
        }
    }

    /// Retry after a failure: read it from the forge now.
    fn pulls_retry(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(facts) = self.areas.pulls.facts.as_mut() {
            facts.read = FactsRead::Loading;
            facts.refreshing = false;
        }
        self.pulls_refresh(cx);
    }
}

/// The pane, for the session the panel shows.
pub(crate) fn pane(
    shell: &Shell,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let facts =
        shell.code_checkout().and_then(|(machine, session, _)| {
            shell.areas.pulls.facts.as_ref().filter(|facts| {
                facts.subject.machine == machine && facts.subject.session == session
            })
        });
    // An answer's "Add to chat" waits for a frame with the window.
    if facts.is_some_and(|facts| !facts.adding.pending.is_empty()) {
        cx.defer_in(window, Shell::pulls_attach_pending);
    }
    let body = match facts.map(|facts| (&facts.read, facts)) {
        None | Some((FactsRead::Loading, _)) => super::pane_skeleton::skeleton(shell, tokens),
        Some((FactsRead::Failed(_), _)) => failed(shell, tokens, cx),
        Some((FactsRead::Ready(view), facts)) => match &view.pull_request {
            Some(pull) => ready(shell, facts, pull, view, tokens, window, cx),
            None => empty(shell, view, tokens),
        },
    };
    div()
        .id("pr-pane")
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(spacing::S0))
        .bg(tokens.neutral.surface_sidebar)
        .child(body)
        .into_any_element()
}

/// The pull request read: one scroll of the toolbar, the header, the
/// checks, a divider and the activity.
fn ready(
    shell: &Shell,
    facts: &Facts,
    pull: &PullRequest,
    view: &PullRequestView,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    div()
        .id("pr-pane-scroll")
        .flex()
        .flex_col()
        .size_full()
        .overflow_y_scroll()
        .child(toolbar(shell, facts, pull, tokens, cx))
        .child(header(shell, pull, tokens))
        .child(super::pane_checks::section(shell, facts, pull, tokens, cx))
        .child(
            div()
                .h(px(DIVIDER))
                .w_full()
                .flex_none()
                .bg(tokens.neutral.border),
        )
        .child(super::pane_activity::section(
            shell,
            facts,
            pull,
            view.feedback,
            tokens,
            window,
            cx,
        ))
        .into_any_element()
}

/// Paseo's toolbar: View at the start, the refresh icon at the end (a
/// spinner while it reads). Paseo names the icon for assistive technology
/// only ("Refresh git and GitHub state"); the kit's controls carry no such
/// name yet, so this one has none either.
fn toolbar(
    shell: &Shell,
    facts: &Facts,
    pull: &PullRequest,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let neutral = tokens.neutral;
    let words = shell.words.pulls();
    let hover = SidebarSurfaces::of(tokens).hover;
    let url = pull.summary.url.clone();
    let view = div()
        .id("pr-pane-view-pr")
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S1))
        .h(px(spacing::S6))
        .px(px(spacing::S1))
        .rounded(px(spacing::S1))
        .type_(Type::Meta)
        .text_color(neutral.foreground)
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .child(glyph(Glyph::ArrowUpRight, 12.0, neutral.foreground_muted))
        .child(words.pane_view().to_owned())
        .on_click(move |_event, _window, cx| cx.open_url(url.as_str()));
    let icon: AnyElement = if facts.refreshing {
        vessel_app_kit::slider::spinner("pr-pane-refreshing", 16.0, neutral.foreground_muted)
            .into_any_element()
    } else {
        glyph(Glyph::RotateCw, 16.0, neutral.foreground_muted).into_any_element()
    };
    let refresh = div()
        .id("pr-pane-refresh")
        .flex()
        .items_center()
        .justify_center()
        .size(px(spacing::S6))
        .rounded(px(spacing::S1))
        .when(!facts.refreshing, |button| {
            button
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .on_click(cx.listener(|shell, _event, _window, cx| shell.pulls_refresh(cx)))
        })
        .child(icon);
    div()
        .flex()
        .flex_row()
        .flex_none()
        .items_center()
        .justify_between()
        .h(px(TOOLBAR))
        .px(px(spacing::S3))
        .bg(neutral.surface0)
        .border_b_1()
        .border_color(neutral.border)
        .child(view)
        .child(refresh)
        .into_any_element()
}

/// Paseo's header: the title at 14 with " #42" muted in the same run, then
/// the state's glyph and word in its colour and the repository; a press
/// opens it on the forge.
fn header(shell: &Shell, pull: &PullRequest, tokens: &Tokens) -> AnyElement {
    let neutral = tokens.neutral;
    let summary = &pull.summary;
    let phase = PullPhase::of(summary);
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
    let url = summary.url.clone();
    let hover = SidebarSurfaces::of(tokens).hover;
    div()
        .id("pr-pane-header")
        .group("pr-pane-header")
        .flex()
        .flex_col()
        .flex_none()
        .gap(px(spacing::S2))
        .p(px(spacing::S4))
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .on_click(move |_event, _window, cx| cx.open_url(url.as_str()))
        .child(
            div()
                .id("pr-pane-title")
                .type_(Type::Body)
                .line_height(px(TITLE_LINE))
                .text_color(neutral.foreground)
                .whitespace_normal()
                .child(StyledText::new(title).with_highlights(number)),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(spacing::S1))
                .min_h(px(spacing::S4))
                .type_(Type::Meta)
                .child(state_glyph_sized(phase, STATE_GLYPH, tokens))
                .child(
                    div()
                        .id("pr-pane-state")
                        .text_color(super::link::state_colour(phase, tokens))
                        .child(phase.words(shell.words).to_owned()),
                )
                .child(
                    div()
                        .ml(px(spacing::S1))
                        .min_w_0()
                        .truncate()
                        .text_color(neutral.foreground_muted)
                        .child(summary.repo.path.as_str().to_owned()),
                )
                .child(
                    div()
                        .ml(px(spacing::S1))
                        .flex_none()
                        .opacity(0.0)
                        .group_hover("pr-pane-header", |style| style.opacity(1.0))
                        .child(glyph(Glyph::ArrowUpRight, 12.0, neutral.foreground_muted)),
                ),
        )
        .into_any_element()
}

/// Paseo's error: its one sentence and Retry, centred.
fn failed(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> AnyElement {
    let words = shell.words.pulls();
    let neutral = tokens.neutral;
    let hover = SidebarSurfaces::of(tokens).hover;
    div()
        .id("pr-pane-error")
        .flex()
        .flex_col()
        .flex_1()
        .items_center()
        .justify_center()
        .gap(px(spacing::S3))
        .px(px(spacing::S4))
        .child(
            div()
                .type_(Type::Meta)
                .text_color(neutral.foreground_muted)
                .text_center()
                .child(words.pane_failed().to_owned()),
        )
        .child(
            div()
                .id("pr-pane-retry")
                .flex()
                .flex_row()
                .items_center()
                .gap(px(spacing::S1))
                .h(px(spacing::S7))
                .px(px(spacing::S2))
                .rounded(px(spacing::S1_5))
                .type_(Type::Meta)
                .text_color(neutral.foreground)
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .child(glyph(Glyph::RotateCw, 12.0, neutral.foreground_muted))
                .child(words.pane_retry().to_owned())
                .on_click(cx.listener(|shell, _event, _window, cx| shell.pulls_retry(cx))),
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
        .id("pull-request-empty-state")
        .flex()
        .flex_col()
        .flex_1()
        .items_center()
        .justify_center()
        .gap(px(spacing::S2))
        .p(px(spacing::S6))
        .text_center()
        .child(
            div()
                .type_(Type::Body)
                .text_color(tokens.neutral.foreground)
                .child(words.empty_title().to_owned()),
        )
        .child(
            div()
                .type_(Type::Body)
                .w(relative(1.0))
                .text_color(tokens.neutral.foreground_muted)
                .child(why),
        )
        .into_any_element()
}
