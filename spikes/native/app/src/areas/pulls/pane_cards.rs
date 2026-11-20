//! The activity's cards: Paseo's `pane.tsx` `SingleActivityCard`,
//! `ReviewCard`, `ThreadBlock` and `ThreadComment` (pinned 0.15.4). A
//! card's header (the author's initial on their identity colour, the
//! author, the verb, the age) folds its body; a review holds the threads
//! its comments started; a thread names its file and lines, and says when
//! it is resolved or outdated.
//!
//! [Decision Log]
//! - Purpose: read what reviewers said, where, in Paseo's cards.
//! - Constraint: the node sends no avatars (ADR 0088 part 3), so every
//!   avatar is Paseo's fallback, the initial on the identity colour.
//! - Choice: Paseo's geometry; the markdown is the kit's renderer.
//! - "Add to chat" closes an open card with words, a review, and a thread
//!   (`pane_attach`), while the session has a composer to take it; a
//!   folded review says it in its header, as Paseo's does.
//! - Each header ends with Paseo's kebab (`pane_menu`: Add to chat, Copy,
//!   Open on GitHub). A press on a card with words folds it (a button
//!   that says whether it is open); one without words opens it on the
//!   forge (a link), as Paseo's do.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::theme::SidebarSurfaces;
use vessel_app_kit::theme::identity::{Hue, fill};
use vessel_app_kit::theme::spacing;
use std::collections::HashSet;

use vessel_app_kit::ui::{
    AnyElement, Context, Div, InteractiveElement as _, IntoElement as _, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_view::workspace::pull::checks::Presentation;
use vessel_app_view::workspace::pull::timeline::{Age, Entry, Thread};
use vessel_app_view::workspace::pull::timeline_words::verb;
use vessel_proto::forge::{ReviewState, TimelineItem};

use crate::shell::Shell;

/// The avatar initial's size (Paseo's `avatarText`).
const INITIAL: f32 = 10.0;
/// A card header's least height (Paseo's `entryHeaderPressable`).
pub(super) const HEADER_MIN: f32 = 36.0;

/// What every card needs.
pub(super) struct Cards<'a> {
    pub(super) shell: &'a Shell,
    pub(super) choices: &'a Choices,
    pub(super) tokens: &'a Tokens,
    pub(super) now: chrono::DateTime<chrono::Utc>,
    /// The session has a composer to add to (Paseo's `attachEnabled`).
    pub(super) attach: bool,
    /// A phone's window: kebabs always show (Paseo's compact form).
    pub(super) touch: bool,
}

/// What a person folded in the pane, for the session read (Paseo keeps
/// these per mounted pane; everything starts open).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Choices {
    pub(crate) checks_closed: bool,
    pub(crate) collapsed: HashSet<Presentation>,
    pub(crate) activity_closed: bool,
    /// Activity entries folded, and those unfolded that start folded.
    pub(crate) folded: HashSet<String>,
    pub(crate) unfolded: HashSet<String>,
}

/// What a card's "Add to chat" adds.
#[derive(Clone)]
pub(super) enum Adds {
    Item(String),
    Thread(String),
}

impl Choices {
    /// Whether the entry `key` is folded (Paseo's `isActivityCollapsed`).
    pub(crate) fn folded(&self, key: &str, at_first: bool) -> bool {
        self.folded.contains(key) || (at_first && !self.unfolded.contains(key))
    }

    /// Fold or unfold `key`.
    pub(crate) fn toggle(&mut self, key: &str, at_first: bool) {
        if self.folded(key, at_first) {
            let _was = self.folded.remove(key);
            let _new = self.unfolded.insert(key.to_owned());
        } else {
            let _was = self.unfolded.remove(key);
            let _new = self.folded.insert(key.to_owned());
        }
    }
}

/// A press that folds `key`: a button named `name` that says whether it
/// is open.
pub(super) fn folds(
    element: gpui::Stateful<Div>,
    key: String,
    at_first: bool,
    (name, open): (String, bool),
    cx: &mut Context<'_, Shell>,
) -> gpui::Stateful<Div> {
    element
        .role(gpui::Role::Button)
        .aria_label(SharedString::from(name))
        .aria_expanded(open)
        .on_click(cx.listener(move |shell, _event, _window, cx| {
            if let Some(facts) = shell.areas.pulls.facts.as_mut() {
                facts.choices.toggle(&key, at_first);
            }
            cx.notify();
        }))
}

impl Cards<'_> {
    /// One entry's card.
    pub(super) fn entry(&self, entry: &Entry<'_>, cx: &mut Context<'_, Shell>) -> AnyElement {
        match entry {
            Entry::Single(item) => self.single(item, cx),
            Entry::Review { review, threads } => self.review(review, threads, cx),
            Entry::Thread(thread) => self
                .card()
                .child(self.thread(thread, cx))
                .into_any_element(),
        }
    }

    /// Paseo's `card`: bordered, rounded 8, 12 in from the sides.
    pub(super) fn card(&self) -> Div {
        div()
            .flex()
            .flex_col()
            .mx(px(spacing::S3))
            .mb(px(spacing::S3))
            .border_1()
            .border_color(self.tokens.neutral.border)
            .rounded(px(spacing::S2))
            .bg(self.tokens.neutral.surface_sidebar)
            .overflow_hidden()
    }

    /// The author's initial on their identity colour (Paseo's fallback
    /// avatar), `across` wide.
    fn avatar(item: &TimelineItem, across: f32) -> AnyElement {
        let login = author(item).unwrap_or_default();
        let initial: String = login
            .chars()
            .next()
            .map(|first| first.to_uppercase().collect())
            .unwrap_or_default();
        div()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(px(across))
            .rounded_full()
            .bg(fill(Hue::of_key(&login.to_lowercase())))
            .text_size(px(INITIAL))
            .text_color(gpui::white())
            .child(initial)
            .into_any_element()
    }

    /// Paseo's `ActivityHeader`: avatar, author, verb, and the age at the
    /// end.
    pub(super) fn header_row(
        &self,
        item: &TimelineItem,
        avatar: f32,
        trailing: Vec<AnyElement>,
    ) -> Div {
        let neutral = self.tokens.neutral;
        let status = self.tokens.status();
        let words = self.shell.words;
        let (mark, ink) = match item {
            TimelineItem::Review {
                state: ReviewState::Approved,
                ..
            } => (Some(Glyph::CircleCheck), status.success.foreground),
            TimelineItem::Review {
                state: ReviewState::ChangesRequested,
                ..
            } => (Some(Glyph::CircleX), status.error.foreground),
            TimelineItem::Review { .. } | TimelineItem::Comment { .. } => {
                (None, neutral.foreground_muted)
            }
        };
        let name = author(item).unwrap_or_else(|| words.pulls().unknown_author().to_owned());
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(spacing::S2))
            .child(Self::avatar(item, avatar))
            .child(
                div()
                    .type_(Type::Body)
                    .text_color(neutral.foreground)
                    .min_w_0()
                    .truncate()
                    .child(name),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_none()
                    .items_center()
                    .gap(px(spacing::S1))
                    .type_(Type::Meta)
                    .text_color(ink)
                    .children(mark.map(|which| glyph(which, 12.0, ink)))
                    .child(verb(item, words).to_owned()),
            )
            .child(
                div()
                    .ml_auto()
                    .flex()
                    .flex_row()
                    .flex_none()
                    .items_center()
                    .gap(px(spacing::S2))
                    .child(
                        div()
                            .type_(Type::Meta)
                            .text_color(neutral.foreground_muted)
                            .child(Age::of(item.created_at(), self.now).words(words)),
                    )
                    .children(trailing),
            )
    }

    /// What a header says, for assistive technology: the author and the
    /// verb ("mkim requested changes").
    pub(super) fn said(&self, item: &TimelineItem) -> String {
        let words = self.shell.words;
        let name = author(item).unwrap_or_else(|| words.pulls().unknown_author().to_owned());
        format!("{name} {}", verb(item, words))
    }

    /// A body as markdown, keyed by its item so its layout is kept.
    pub(super) fn markdown(item: &TimelineItem) -> Option<AnyElement> {
        let text = body(item);
        (!text.trim().is_empty()).then(|| {
            vessel_app_kit::markdown(
                SharedString::from(format!("pr-pane-md-{}", item.id().as_str())),
                text.to_owned(),
            )
            .into_any_element()
        })
    }

    /// A comment or a review with no thread of its own.
    fn single(&self, item: &TimelineItem, cx: &mut Context<'_, Shell>) -> AnyElement {
        let hover = SidebarSurfaces::of(self.tokens).hover;
        let key = item.id().as_str().to_owned();
        let id = SharedString::from(format!("pr-pane-activity-{key}"));
        let group = SharedString::from(format!("pr-pane-card-{key}"));
        let kebab: Vec<AnyElement> = self.item_kebab(item, &group, cx).into_iter().collect();
        let Some(text) = Self::markdown(item) else {
            // Nothing to unfold: a row that opens it on the forge.
            let url = item_url(item);
            return div()
                .id(id)
                .group(group)
                .mx(px(spacing::S3))
                .mb(px(spacing::S2))
                .px(px(spacing::S3))
                .py(px(spacing::S2))
                .min_h(px(HEADER_MIN))
                .rounded(px(spacing::S2))
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .role(gpui::Role::Link)
                .aria_label(SharedString::from(self.said(item)))
                .on_click(move |_event, _window, cx| {
                    if let Some(url) = &url {
                        cx.open_url(url);
                    }
                })
                .child(self.header_row(item, 20.0, kebab))
                .into_any_element();
        };
        let folded = self.choices.folded(&key, false);
        let head = folds(
            div()
                .id(id)
                .px(px(spacing::S3))
                .py(px(spacing::S2))
                .min_h(px(HEADER_MIN))
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .child(self.header_row(item, 20.0, kebab)),
            key,
            false,
            (self.said(item), !folded),
            cx,
        );
        let mut card = self.card().group(group).child(head);
        if !folded {
            card = card
                .child(
                    div()
                        .px(px(spacing::S3))
                        .pt(px(spacing::S2))
                        .pb(px(spacing::S3))
                        .child(text),
                )
                .children(
                    (self.attach && super::pane_attach::addable(item))
                        .then(|| self.footer(Adds::Item(item.id().as_str().to_owned()), cx)),
                );
        }
        card.into_any_element()
    }

    /// Paseo's `cardFooter`: "Add to chat" at the start.
    pub(super) fn footer(&self, adds: Adds, cx: &mut Context<'_, Shell>) -> AnyElement {
        let id = match &adds {
            Adds::Item(id) | Adds::Thread(id) => SharedString::from(format!("pr-pane-add-{id}")),
        };
        div()
            .flex()
            .flex_row()
            .pl(px(spacing::S2))
            .pr(px(spacing::S2))
            .pb(px(spacing::S2))
            .child(
                super::pane_checks::add_button(
                    id,
                    self.shell.words.pulls().add_to_chat(),
                    false,
                    self.tokens,
                )
                .on_click(cx.listener(
                    move |shell, _event, window, cx| match &adds {
                        Adds::Item(id) => shell.pulls_add_item(id, window, cx),
                        Adds::Thread(key) => shell.pulls_add_thread(key, window, cx),
                    },
                )),
            )
            .into_any_element()
    }

    /// A review and its threads; folded, its header keeps Add to chat
    /// and the count of its threads (Paseo's `ReviewCard`).
    fn review(
        &self,
        review: &TimelineItem,
        threads: &[Thread<'_>],
        cx: &mut Context<'_, Shell>,
    ) -> AnyElement {
        let hover = SidebarSurfaces::of(self.tokens).hover;
        let key = review.id().as_str().to_owned();
        let group = SharedString::from(format!("pr-pane-card-{key}"));
        let folded = self.choices.folded(&key, false);
        let addable = self.attach && super::pane_attach::addable(review);
        let mut trailing = Vec::new();
        if folded && addable {
            trailing.push(self.header_add(review.id().as_str().to_owned(), cx));
        }
        if folded {
            trailing.push(self.count_chip(threads.len()));
        }
        trailing.extend(self.item_kebab(review, &group, cx));
        let head = folds(
            div()
                .id(SharedString::from(format!("pr-pane-review-{key}")))
                .px(px(spacing::S3))
                .py(px(spacing::S2))
                .min_h(px(HEADER_MIN))
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .child(self.header_row(review, 20.0, trailing)),
            key,
            false,
            (self.said(review), !folded),
            cx,
        );
        let mut card = self.card().group(group).child(head);
        if !folded {
            card = card
                .children(Self::markdown(review).map(|text| {
                    div()
                        .px(px(spacing::S3))
                        .pt(px(spacing::S2))
                        .pb(px(spacing::S3))
                        .child(text)
                }))
                .children(
                    addable.then(|| self.footer(Adds::Item(review.id().as_str().to_owned()), cx)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .pt(px(spacing::S1))
                        .pb(px(spacing::S1))
                        .children(threads.iter().map(|thread| {
                            div()
                                .mx(px(spacing::S3))
                                .mb(px(spacing::S2))
                                .border_1()
                                .border_color(self.tokens.neutral.border)
                                .rounded(px(spacing::S1_5))
                                .overflow_hidden()
                                .child(self.thread(thread, cx))
                        })),
                );
        }
        card.into_any_element()
    }

    /// A folded review's "Add to chat" in its header (Paseo's ghost
    /// button); its press does not unfold the card.
    fn header_add(&self, item: String, cx: &mut Context<'_, Shell>) -> AnyElement {
        super::pane_checks::add_button(
            SharedString::from(format!("pr-pane-head-add-{item}")),
            self.shell.words.pulls().add_to_chat(),
            false,
            self.tokens,
        )
        .on_click(cx.listener(move |shell, _event, window, cx| {
            cx.stop_propagation();
            shell.pulls_add_item(&item, window, cx);
        }))
        .into_any_element()
    }
}

/// An item's author's login.
pub(super) fn author(item: &TimelineItem) -> Option<String> {
    match item {
        TimelineItem::Review { author, .. } | TimelineItem::Comment { author, .. } => {
            author.as_ref().map(|login| login.as_str().to_owned())
        }
    }
}

/// An item's words.
fn body(item: &TimelineItem) -> &str {
    match item {
        TimelineItem::Review { body, .. } | TimelineItem::Comment { body, .. } => body.as_str(),
    }
}

/// An item's page.
fn item_url(item: &TimelineItem) -> Option<String> {
    match item {
        TimelineItem::Review { url, .. } | TimelineItem::Comment { url, .. } => {
            url.as_ref().map(|url| url.as_str().to_owned())
        }
    }
}
