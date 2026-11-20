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
//!   (`pane_attach`), while the session has a composer to take it.
//! - Difference: Paseo's per-card menu (Add to chat, Copy, Open on
//!   GitHub) is not drawn; a press on a card without words opens it on
//!   the forge, as Paseo's does.

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::theme::SidebarSurfaces;
use vessel_app_kit::theme::identity::{Hue, fill};
use vessel_app_kit::ui::{
    AnyElement, Context, Div, Hsla, InteractiveElement as _, IntoElement as _, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_view::workspace::pull::timeline::{Age, Entry, Thread, thread_path};
use vessel_app_view::workspace::pull::timeline_words::verb;
use vessel_proto::forge::{ReviewState, TimelineItem};

use super::Choices;
use crate::shell::Shell;

/// What every card needs.
pub(super) struct Cards<'a> {
    pub(super) shell: &'a Shell,
    pub(super) choices: &'a Choices,
    pub(super) tokens: &'a Tokens,
    pub(super) now: chrono::DateTime<chrono::Utc>,
    /// The session has a composer to add to (Paseo's `attachEnabled`).
    pub(super) attach: bool,
}

/// What a card's "Add to chat" adds.
#[derive(Clone)]
enum Adds {
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

/// A press that folds `key`.
fn folds(
    element: gpui::Stateful<Div>,
    key: String,
    at_first: bool,
    cx: &mut Context<'_, Shell>,
) -> gpui::Stateful<Div> {
    element.on_click(cx.listener(move |shell, _event, _window, cx| {
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
    fn card(&self) -> Div {
        div()
            .flex()
            .flex_col()
            .mx(px(12.0))
            .mb(px(12.0))
            .border_1()
            .border_color(self.tokens.neutral.border)
            .rounded(px(8.0))
            .bg(self.tokens.neutral.surface_sidebar)
            .overflow_hidden()
    }

    /// The author's initial on their identity colour (Paseo's fallback
    /// avatar), `across` wide.
    fn avatar(&self, item: &TimelineItem, across: f32) -> AnyElement {
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
            .text_size(px(10.0))
            .text_color(gpui::white())
            .child(initial)
            .into_any_element()
    }

    /// Paseo's `ActivityHeader`: avatar, author, verb, and the age at the
    /// end.
    fn header_row(&self, item: &TimelineItem, avatar: f32, extra: Option<AnyElement>) -> Div {
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
            .gap(px(8.0))
            .child(self.avatar(item, avatar))
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
                    .gap(px(4.0))
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
                    .gap(px(8.0))
                    .child(
                        div()
                            .type_(Type::Meta)
                            .text_color(neutral.foreground_muted)
                            .child(Age::of(item.created_at(), self.now).words(words)),
                    )
                    .children(extra),
            )
    }

    /// A body as markdown, keyed by its item so its layout is kept.
    fn body(&self, item: &TimelineItem) -> Option<AnyElement> {
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
        let Some(text) = self.body(item) else {
            // Nothing to unfold: a row that opens it on the forge.
            let url = item_url(item);
            return div()
                .id(id)
                .mx(px(12.0))
                .mb(px(8.0))
                .px(px(12.0))
                .py(px(8.0))
                .min_h(px(36.0))
                .rounded(px(8.0))
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .on_click(move |_event, _window, cx| {
                    if let Some(url) = &url {
                        cx.open_url(url);
                    }
                })
                .child(self.header_row(item, 20.0, None))
                .into_any_element();
        };
        let folded = self.choices.folded(&key, false);
        let head = folds(
            div()
                .id(id)
                .px(px(12.0))
                .py(px(8.0))
                .min_h(px(36.0))
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .child(self.header_row(item, 20.0, None)),
            key,
            false,
            cx,
        );
        let mut card = self.card().child(head);
        if !folded {
            card = card
                .child(div().px(px(12.0)).pt(px(8.0)).pb(px(12.0)).child(text))
                .children(
                    (self.attach && super::pane_attach::addable(item))
                        .then(|| self.footer(Adds::Item(item.id().as_str().to_owned()), cx)),
                );
        }
        card.into_any_element()
    }

    /// Paseo's `cardFooter`: "Add to chat" at the start.
    fn footer(&self, adds: Adds, cx: &mut Context<'_, Shell>) -> AnyElement {
        let id = match &adds {
            Adds::Item(id) | Adds::Thread(id) => SharedString::from(format!("pr-pane-add-{id}")),
        };
        div()
            .flex()
            .flex_row()
            .pl(px(8.0))
            .pr(px(8.0))
            .pb(px(8.0))
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

    /// A review and its threads.
    fn review(
        &self,
        review: &TimelineItem,
        threads: &[Thread<'_>],
        cx: &mut Context<'_, Shell>,
    ) -> AnyElement {
        let hover = SidebarSurfaces::of(self.tokens).hover;
        let key = review.id().as_str().to_owned();
        let folded = self.choices.folded(&key, false);
        let chip = folded.then(|| self.count_chip(threads.len()));
        let head = folds(
            div()
                .id(SharedString::from(format!("pr-pane-review-{key}")))
                .px(px(12.0))
                .py(px(8.0))
                .min_h(px(36.0))
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .child(self.header_row(review, 20.0, chip)),
            key,
            false,
            cx,
        );
        let mut card = self.card().child(head);
        if !folded {
            card = card
                .children(
                    self.body(review)
                        .map(|text| div().px(px(12.0)).pt(px(8.0)).pb(px(12.0)).child(text)),
                )
                .children(
                    (self.attach && super::pane_attach::addable(review))
                        .then(|| self.footer(Adds::Item(review.id().as_str().to_owned()), cx)),
                )
                .child(div().flex().flex_col().pt(px(4.0)).pb(px(4.0)).children(
                    threads.iter().map(|thread| {
                        div()
                            .mx(px(12.0))
                            .mb(px(8.0))
                            .border_1()
                            .border_color(self.tokens.neutral.border)
                            .rounded(px(6.0))
                            .overflow_hidden()
                            .child(self.thread(thread, cx))
                    }),
                ));
        }
        card.into_any_element()
    }

    /// A count of comments or threads (Paseo's chip: the message glyph at
    /// 11 and the number).
    fn count_chip(&self, count: usize) -> AnyElement {
        let muted = self.tokens.neutral.foreground_muted;
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(3.0))
            .type_(Type::Meta)
            .text_color(muted)
            .child(glyph(Glyph::Message, 11.0, muted))
            .child(count.to_string())
            .into_any_element()
    }

    /// A badge (Paseo's `StatusBadge`): "Resolved" in the success tint,
    /// "Outdated" muted.
    fn badge(&self, text: &str, resolved: bool) -> AnyElement {
        let neutral = self.tokens.neutral;
        let success = self.tokens.status().success;
        let (back, edge, ink): (Hsla, Hsla, Hsla) = if resolved {
            (success.tint, gpui::transparent_black(), success.foreground)
        } else {
            (neutral.surface3, neutral.border, neutral.foreground_muted)
        };
        div()
            .flex_none()
            .px(px(8.0))
            .py(px(3.0))
            .rounded_full()
            .border_1()
            .border_color(edge)
            .bg(back)
            .type_(Type::Meta)
            .text_color(ink)
            .child(text.to_owned())
            .into_any_element()
    }

    /// Paseo's `ThreadBlock`: the header (path, badges, a count while
    /// folded), then the root comment and its replies.
    fn thread(&self, thread: &Thread<'_>, cx: &mut Context<'_, Shell>) -> AnyElement {
        let neutral = self.tokens.neutral;
        let words = self.shell.words.pulls();
        let hover = SidebarSurfaces::of(self.tokens).hover;
        let key = thread.key();
        let at_first = thread.folded_at_first();
        let folded = self.choices.folded(&key, at_first);
        let path = thread_path(thread.place);
        let label = path.unwrap_or_else(|| words.thread_discussion().to_owned());
        let head = folds(
            div()
                .id(SharedString::from(format!("pr-pane-{key}")))
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .px(px(12.0))
                .py(px(8.0))
                .min_h(px(36.0))
                .bg(neutral.surface1)
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .child(
                    div()
                        .type_(Type::Mono)
                        .text_color(neutral.foreground)
                        .min_w_0()
                        .truncate()
                        .child(label),
                )
                .children(
                    thread
                        .place
                        .resolved
                        .then(|| self.badge(words.thread_resolved(), true)),
                )
                .children(
                    thread
                        .place
                        .outdated
                        .then(|| self.badge(words.thread_outdated(), false)),
                )
                .children(folded.then(|| {
                    div()
                        .ml_auto()
                        .child(self.count_chip(thread.comments.len()))
                })),
            key,
            at_first,
            cx,
        );
        let mut block = div().flex().flex_col().child(head);
        if !folded {
            let (root, replies) = match thread.comments.split_first() {
                Some((root, replies)) => (Some(*root), replies),
                None => (None, &[][..]),
            };
            block = block.children(root.map(|root| self.thread_comment(root)));
            // Paseo's `replyRail`: 12 in, 8 below the root, each reply a
            // rounded card.
            if !replies.is_empty() {
                block = block.child(div().flex().flex_col().mx(px(12.0)).mt(px(8.0)).children(
                    replies.iter().map(|reply| {
                        div()
                            .mb(px(8.0))
                            .border_1()
                            .border_color(neutral.border)
                            .rounded(px(6.0))
                            .bg(neutral.surface1)
                            .overflow_hidden()
                            .child(self.thread_comment(reply))
                    }),
                ));
            }
            if self.attach {
                block = block.child(self.footer(Adds::Thread(thread.key()), cx));
            }
        }
        block.into_any_element()
    }

    /// Paseo's `ThreadComment`: a line above, the 16 avatar header, then
    /// the words.
    fn thread_comment(&self, item: &TimelineItem) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .border_t_1()
            .border_color(self.tokens.neutral.border)
            .pb(px(8.0))
            .child(
                div()
                    .px(px(12.0))
                    .pt(px(8.0))
                    .pb(px(4.0))
                    .min_h(px(32.0))
                    .child(self.header_row(item, 16.0, None)),
            )
            .children(
                self.body(item)
                    .map(|text| div().px(px(12.0)).pt(px(8.0)).child(text)),
            )
            .into_any_element()
    }
}

/// An item's author's login.
fn author(item: &TimelineItem) -> Option<String> {
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
