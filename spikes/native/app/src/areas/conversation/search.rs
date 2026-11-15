//! The palette's Messages group (GOAT's `CommandPaletteThreadItems` and
//! `ThreadSearchMatch.tsx`): every reachable machine searched as the
//! person types, a row per message that matched, and opening one at that
//! message with find on the words. The palette (its field, its keys, its
//! place for the group) is the shell's (track S3); the search, the rows
//! and opening a hit are this module's seams.
//!
//! [Decision Log]
//! - Purpose: find what was said in any session, on any machine, from the
//!   palette (protocol 12's `session.search`).
//! - The reference: GOAT's row is a thread row whose project line gives
//!   way to the message: "You:" in the info blue or "Agent:" in the
//!   success green, then the snippet, its matches semibold in the
//!   foreground and the rest muted; the session's title above with its
//!   place after it, and when the message was written at the right.
//!   Choosing it opens the thread with find open on the words at that
//!   message.
//! - Choice: the same row and the same opening. The search starts once
//!   the words have rested 150 ms (each keystroke would otherwise ask
//!   every machine, and a node runs only two searches at once); the
//!   merging, paging and notes are the view's (`view::search`).
//! - Seams for the palette: [`Shell::message_search_query`] as the words
//!   change (empty when it closes); [`Shell::message_search_rows`], the
//!   rows top to bottom, by typed id; [`message_row`] draws one;
//!   [`message_search_note`] the lines under them; choosing a row calls
//!   [`Shell::open_search_hit`] or [`Shell::message_search_more`].
//! - Cost: the place shown is the machine only (GOAT adds the project; a
//!   card here does not name one).

use std::time::Duration;

use chrono::Utc;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::theme::spacing;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, FluentBuilder as _, HighlightStyle, IntoElement as _,
    ParentElement as _, Styled as _, StyledText, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _};
use vessel_app_link::Command;
use vessel_app_link::search::{Answer as Searched, Ask, Refusal};
use vessel_app_view::Card;
use vessel_app_view::ago::Ago;
use vessel_app_view::board::Reach;
use vessel_app_view::connection::ConnectionContext;
use vessel_app_view::search::{Answer, HitId, MessageSearch, Note, PageAsk, Row, SearchFailure};
use vessel_proto::EventSeq;
use vessel_proto::search::{MessageRole, SearchHit, SearchQuery};

use crate::areas::session::Open;
use crate::shell::Shell;

#[cfg(test)]
mod residency_tests;

/// How long the words rest before they are searched.
const SETTLE: Duration = Duration::from_millis(150);

/// The palette's message search: the view's state and the words typed.
#[derive(Debug, Default)]
pub(crate) struct Searching {
    view: MessageSearch,
    /// Admission is fixed for the whole paged query.
    sources: Vec<(String, Option<ConnectionContext>)>,
}

impl Shell {
    /// Invalidate old rows and replies immediately; debounce only the node work.
    pub(crate) fn message_search_query(&mut self, text: &str, cx: &mut Context<'_, Self>) {
        let sources = search_sources(&self.fleet);
        let machines = sources.iter().map(|(name, _)| name.as_str());
        let asks = self
            .areas
            .palette
            .messages_search
            .view
            .start(text, machines);
        self.areas.palette.messages_search.sources = sources;
        let ticket = self.areas.palette.messages_search.view.ticket();
        cx.notify();
        if asks.is_empty() {
            return;
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SETTLE).await;
            let _gone = this.update(cx, |shell, _cx| {
                if shell.areas.palette.messages_search.view.ticket() == ticket {
                    shell.message_search_ask(asks);
                }
            });
        })
        .detach();
    }

    /// Send `asks`, tagged with the search they belong to.
    fn message_search_ask(&self, asks: Vec<PageAsk>) {
        let ticket = self.areas.palette.messages_search.view.ticket();
        for ask in asks {
            self.command(Command::Search(Ask {
                connection: self
                    .areas
                    .palette
                    .messages_search
                    .sources
                    .iter()
                    .find(|(name, _)| name == &ask.machine)
                    .and_then(|(_, context)| context.clone()),
                machine: ask.machine,
                ticket,
                request: ask.request,
            }));
        }
    }

    /// Changed admission invalidates every old page/cursor before a replacement query.
    pub(crate) fn message_search_fleet_changed(&mut self, cx: &mut Context<'_, Self>) {
        let search = &self.areas.palette.messages_search;
        if search.sources == search_sources(&self.fleet) {
            return;
        }
        if let Some(query) = search.view.query().map(str::to_owned) {
            self.message_search_query(&query, cx);
        }
    }

    /// A machine answered a search.
    pub(crate) fn message_searched(&mut self, answer: Searched, cx: &mut Context<'_, Self>) {
        let page = match answer.result {
            Ok(page) => Answer::Page(page),
            Err(Refusal::Older) => Answer::Older,
            Err(Refusal::NotConnected) => Answer::NotConnected,
            Err(Refusal::TimedOut) => Answer::Failed(SearchFailure::TimedOut),
            Err(Refusal::Failed { .. }) => Answer::Failed(SearchFailure::Failed),
        };
        let asks =
            self.areas
                .palette
                .messages_search
                .view
                .answered(answer.ticket, &answer.machine, page);
        self.message_search_ask(asks);
        cx.notify();
    }

    /// The group's rows, top to bottom (none while no words are searched).
    pub(crate) fn message_search_rows(&self) -> Vec<Row> {
        self.areas.palette.messages_search.view.rows()
    }

    /// The person chose More: show the next messages.
    pub(crate) fn message_search_more(&mut self, cx: &mut Context<'_, Self>) {
        let asks = self.areas.palette.messages_search.view.more();
        self.message_search_ask(asks);
        cx.notify();
    }

    /// Prepare before clearing the palette; stale or removed rows cannot navigate.
    pub(crate) fn message_search_destination(&self, hit: &HitId) -> Option<Destination> {
        let view = &self.areas.palette.messages_search.view;
        let found = view.hit(hit)?;
        let source = self
            .areas
            .palette
            .messages_search
            .sources
            .iter()
            .find(|(name, _)| name == &hit.machine)?;
        let current = self
            .fleet
            .machines
            .iter()
            .find(|machine| machine.name == hit.machine)?;
        if current.reach != Reach::Online || source.1 != current.connection_context() {
            return None;
        }
        Some(Destination {
            open: card_of(self, hit).map(Open::of)?,
            seq: found.seq,
            query: view.query_value()?.clone(),
        })
    }

    /// Open with the query retained before closing invalidated the result set.
    pub(crate) fn open_search_hit(
        &mut self,
        destination: Destination,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.open_session(destination.open, window, cx);
        self.conversation_reveal(destination.seq, cx);
        self.find_at(destination.query.as_str(), destination.seq, window, cx);
    }
}

fn search_sources(fleet: &vessel_app_view::Fleet) -> Vec<(String, Option<ConnectionContext>)> {
    fleet
        .machines
        .iter()
        .filter(|machine| machine.reach == Reach::Online)
        .map(|machine| (machine.name.clone(), machine.connection_context()))
        .collect()
}

/// A prepared selection cannot lose its query when the palette closes.
pub(crate) struct Destination {
    open: Open,
    seq: EventSeq,
    query: SearchQuery,
}

/// The card of `hit`'s session, when its machine lists it.
fn card_of<'a>(shell: &'a Shell, hit: &HitId) -> Option<&'a Card> {
    shell
        .fleet
        .machines
        .iter()
        .find(|machine| machine.name == hit.machine)?
        .sessions
        .iter()
        .find(|card| card.session == hit.session)
}

/// One row of the group: a message (GOAT's message row), or More.
/// `active`: the palette's keys are on it (Paseo's `surface2`);
/// `compact`: a phone's width (GOAT's taller row and larger text there).
pub(crate) fn message_row(
    shell: &Shell,
    row: &Row,
    active: bool,
    compact: bool,
    tokens: &Tokens,
) -> AnyElement {
    let neutral = tokens.neutral;
    let words = shell.t.lang.conversation();
    let frame = div()
        .flex()
        .flex_row()
        .gap(px(spacing::S2))
        .min_h(px(if compact {
            vessel_app_kit::breakpoint::TOUCH_TARGET
        } else {
            spacing::S7
        }))
        .px(px(spacing::S2))
        .py(px(spacing::S1_5))
        .rounded(px(spacing::S1))
        .when(active, |row| row.bg(neutral.surface2));
    let hit = match row {
        Row::More => {
            return frame
                .items_center()
                .child(glyph(Glyph::ChevronDown, 16.0, neutral.foreground_muted))
                .child(
                    div()
                        .text_size(px(spacing::S3_5))
                        .text_color(neutral.foreground_muted)
                        .child(words.search_more()),
                )
                .into_any_element();
        }
        Row::Hit(id) => id,
    };
    let Some(found) = shell.areas.palette.messages_search.view.hit(hit) else {
        return frame.into_any_element();
    };
    let title = card_of(shell, hit).map_or_else(
        || hit.session.to_string(),
        |card| shell.t.title(&card.title).to_owned(),
    );
    let when = shell.t.lang.machines().ago(Ago::of(found.at, Utc::now()));
    let head = head(title, &hit.machine, tokens);
    frame
        .items_start()
        .child(div().flex_none().pt(px(spacing::S0_5)).child(glyph(
            Glyph::Message,
            16.0,
            neutral.foreground_muted,
        )))
        .child(
            div()
                .flex_1()
                .min_w(px(spacing::S0))
                .flex()
                .flex_col()
                .child(head)
                .child(excerpt(shell, found, tokens)),
        )
        .child(
            div()
                .flex_none()
                .min_w(px(spacing::S12))
                .text_right()
                .text_size(px(spacing::S3))
                .text_color(neutral.foreground_extra_muted)
                .child(when),
        )
        .into_any_element()
}

/// A message row's head: its session's title, then its machine, quieter.
fn head(title: String, machine: &str, tokens: &Tokens) -> Div {
    let neutral = tokens.neutral;
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(spacing::S1_5))
        .min_w(px(spacing::S0))
        .text_size(px(spacing::S3_5))
        .child(
            div()
                .min_w(px(spacing::S0))
                .truncate()
                .text_color(neutral.foreground)
                .child(title),
        )
        .child(
            div()
                .flex_none()
                .max_w(gpui::relative(0.4))
                .truncate()
                .text_size(px(spacing::S3))
                .text_color(neutral.foreground_extra_muted)
                .child(machine.to_owned()),
        )
}

/// GOAT's excerpt: who said it, then the snippet with its matches
/// semibold in the foreground, on one line.
fn excerpt(shell: &Shell, hit: &SearchHit, tokens: &Tokens) -> AnyElement {
    let words = shell.t.lang.conversation();
    let status = tokens.status();
    let neutral = tokens.neutral;
    let (who, ink) = match hit.role {
        MessageRole::Prompt => (words.search_you(), status.running_dot),
        MessageRole::Reply => (words.search_agent(), status.success.foreground),
    };
    let mut text = String::from(who);
    let mut marks = vec![(
        0..text.len(),
        HighlightStyle {
            color: Some(ink),
            ..HighlightStyle::default()
        },
    )];
    text.push(' ');
    if hit.snippet.cut_before {
        text.push('…');
    }
    for part in &hit.snippet.parts {
        let start = text.len();
        text.push_str(&part.text);
        if part.matched {
            marks.push((
                start..text.len(),
                HighlightStyle {
                    color: Some(neutral.foreground),
                    font_weight: Some(gpui::FontWeight::SEMIBOLD),
                    ..HighlightStyle::default()
                },
            ));
        }
    }
    if hit.snippet.cut_after {
        text.push('…');
    }
    div()
        .min_w(px(spacing::S0))
        .truncate()
        .text_size(px(spacing::S3))
        .text_color(neutral.foreground_muted)
        .child(StyledText::new(text).with_highlights(marks))
        .into_any_element()
}

/// The lines under the group's rows: searching, no match, the words not a
/// search, machines too old to search, machines that could not.
pub(crate) fn message_search_note(shell: &Shell, tokens: &Tokens) -> Option<AnyElement> {
    let notes = shell.areas.palette.messages_search.view.notes();
    if notes.is_empty() {
        return None;
    }
    let words = shell.t.lang.conversation();
    let lines = notes.into_iter().flat_map(|note| match note {
        Note::Invalid => vec![words.search_invalid().to_owned()],
        Note::Searching => vec![words.search_searching().to_owned()],
        Note::Nothing => vec![words.search_nothing().to_owned()],
        Note::Older(machines) => vec![words.search_older(&machines)],
        Note::Failed(failed) => failed
            .iter()
            .map(|(machine, why)| words.search_failed(machine, failure_text(shell, *why)))
            .collect(),
    });
    Some(
        div()
            .flex()
            .flex_col()
            .gap(px(spacing::S0_5))
            .px(px(spacing::S2))
            .py(px(spacing::S1_5))
            .type_(Type::Meta)
            .text_color(tokens.neutral.foreground_muted)
            .children(lines.map(|line| div().child(line)))
            .into_any_element(),
    )
}

/// Render the fact in the current prepared language, never the arrival language.
fn failure_text(shell: &Shell, failure: SearchFailure) -> &str {
    let words = shell.words.message_search();
    match failure {
        SearchFailure::TimedOut => words.timeout(),
        SearchFailure::Failed => words.failed(),
    }
}
