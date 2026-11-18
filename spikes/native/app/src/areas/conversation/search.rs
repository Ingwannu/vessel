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
//! - Until the palette draws the group, [`example`] does (the scroll
//!   bench's way: only when `VESSEL_APP_MESSAGE_SEARCH` names words, a
//!   palette-sized card over the session shown), so the seams have a
//!   caller and the rows can be checked on a fleet. It goes when the
//!   palette's group lands.
//! - Cost: the place shown is the machine only (GOAT adds the project; a
//!   card here does not name one).

use std::cell::Cell;
use std::time::Duration;

use chrono::Utc;
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::picker::glyph;
use vessel_app_kit::ui::{
    AnyElement, Context, Div, ElementId, FluentBuilder as _, HighlightStyle,
    InteractiveElement as _, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Styled as _, StyledText, Window, div, px,
};
use vessel_app_kit::{Tokens, Type, TypeExt as _, breakpoint, popup};
use vessel_app_link::Command;
use vessel_app_link::search::{Answer as Searched, Ask, Refusal};
use vessel_app_view::Card;
use vessel_app_view::ago::Ago;
use vessel_app_view::board::Reach;
use vessel_app_view::search::{Answer, HitId, MessageSearch, Note, PageAsk, Row};
use vessel_proto::search::{MessageRole, SearchHit};

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
    /// Keystrokes seen, so a search starts only for the last words.
    typed: u64,
    /// [`example`]'s words were searched (set while drawing).
    example_started: Cell<bool>,
    /// [`example`] was closed by choosing a row.
    example_closed: bool,
}

impl Shell {
    /// The palette's words changed (empty: it closed or was cleared):
    /// search them once they rest.
    pub(crate) fn message_search_query(&mut self, text: &str, cx: &mut Context<'_, Self>) {
        let searching = &mut self.areas.palette.messages_search;
        searching.typed = searching.typed.wrapping_add(1);
        let typed = searching.typed;
        if text.trim().is_empty() {
            let _nothing = searching.view.start("", std::iter::empty());
            cx.notify();
            return;
        }
        let text = text.to_owned();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SETTLE).await;
            let _gone = this.update(cx, |shell, cx| {
                if shell.areas.palette.messages_search.typed == typed {
                    shell.message_search_start(&text, cx);
                }
            });
        })
        .detach();
    }

    /// Search `text` on every machine the app reaches.
    fn message_search_start(&mut self, text: &str, cx: &mut Context<'_, Self>) {
        let machines: Vec<String> = self
            .fleet
            .machines
            .iter()
            .filter(|machine| machine.reach == Reach::Online)
            .map(|machine| machine.name.clone())
            .collect();
        let asks = self
            .areas
            .palette
            .messages_search
            .view
            .start(text, machines.iter().map(String::as_str));
        self.message_search_ask(asks);
        cx.notify();
    }

    /// Send `asks`, tagged with the search they belong to.
    fn message_search_ask(&self, asks: Vec<PageAsk>) {
        let ticket = self.areas.palette.messages_search.view.ticket();
        for ask in asks {
            self.command(Command::Search(Ask {
                machine: ask.machine,
                ticket,
                request: ask.request,
            }));
        }
    }

    /// A machine answered a search.
    pub(crate) fn message_searched(&mut self, answer: Searched, cx: &mut Context<'_, Self>) {
        let page = match answer.result {
            Ok(page) => Answer::Page(page),
            Err(Refusal::Older) => Answer::Older,
            Err(Refusal::NotConnected) => Answer::NotConnected,
            Err(Refusal::Failed { message, .. }) => Answer::Failed(message),
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

    /// Open the session of `hit` at its message, with find open on the
    /// words searched (the palette closes itself first). Nothing when the
    /// machine no longer lists the session.
    pub(crate) fn open_search_hit(
        &mut self,
        hit: &HitId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(open) = card_of(self, hit).map(Open::of) else {
            tracing::debug!(session = %hit.session, "a search hit's session is not listed");
            return;
        };
        let query = self
            .areas
            .palette
            .messages_search
            .view
            .query()
            .map(str::to_owned);
        self.open_session(open, window, cx);
        self.conversation_reveal(hit.seq, cx);
        if let Some(query) = query {
            self.find_at(&query, hit.seq, window, cx);
        }
    }
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
        .gap(px(8.0))
        .min_h(px(if compact { 32.0 } else { 28.0 }))
        .px(px(8.0))
        .py(px(6.0))
        .rounded(px(4.0))
        .when(active, |row| row.bg(neutral.surface2));
    let hit = match row {
        Row::More => {
            return frame
                .items_center()
                .child(glyph(Glyph::ChevronDown, 16.0, neutral.foreground_muted))
                .child(
                    div()
                        .text_size(px(14.0))
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
        .child(div().flex_none().pt(px(2.0)).child(glyph(
            Glyph::Message,
            16.0,
            neutral.foreground_muted,
        )))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .child(head)
                .child(excerpt(shell, found, tokens)),
        )
        .child(
            div()
                .flex_none()
                .min_w(px(48.0))
                .text_right()
                .text_size(px(12.0))
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
        .gap(px(6.0))
        .min_w(px(0.0))
        .text_size(px(14.0))
        .child(
            div()
                .min_w(px(0.0))
                .truncate()
                .text_color(neutral.foreground)
                .child(title),
        )
        .child(
            div()
                .flex_none()
                .max_w(px(160.0))
                .truncate()
                .text_size(px(12.0))
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
        .min_w(px(0.0))
        .truncate()
        .text_size(px(12.0))
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
            .map(|(machine, why)| words.search_failed(machine, why))
            .collect(),
    });
    Some(
        div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .px(px(8.0))
            .py(px(6.0))
            .type_(Type::Meta)
            .text_color(tokens.neutral.foreground_muted)
            .children(lines.map(|line| div().child(line)))
            .into_any_element(),
    )
}

/// The words [`example`] searches, from `VESSEL_APP_MESSAGE_SEARCH`.
fn example_words() -> Option<String> {
    std::env::var("VESSEL_APP_MESSAGE_SEARCH")
        .ok()
        .filter(|words| !words.trim().is_empty())
}

/// The Messages group in a palette-sized card over the session shown,
/// until the palette draws it: only with `VESSEL_APP_MESSAGE_SEARCH` set,
/// for checking the rows on a fleet. Its first row is drawn as the one
/// the keys are on; a press chooses a row as the palette's Enter will.
pub(crate) fn example(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let words = example_words()?;
    let searching = &shell.areas.palette.messages_search;
    if searching.example_closed {
        return None;
    }
    if !searching.example_started.get() {
        searching.example_started.set(true);
        cx.defer_in(window, move |shell, _window, cx| {
            shell.message_search_query(&words, cx);
        });
    }
    let neutral = tokens.neutral;
    let compact = breakpoint::phone(window);
    let rows = shell.message_search_rows();
    let listed = rows.iter().enumerate().map(|(index, row)| {
        let chosen = row.clone();
        div()
            .id(ElementId::NamedInteger(
                "message-search-row".into(),
                super::entries::id(index),
            ))
            .cursor_pointer()
            .on_click(cx.listener(move |shell, _event, window, cx| match &chosen {
                Row::Hit(hit) => {
                    shell.areas.palette.messages_search.example_closed = true;
                    shell.open_search_hit(hit, window, cx);
                }
                Row::More => shell.message_search_more(cx),
            }))
            .child(message_row(shell, row, index == 0, compact, tokens))
    });
    // As wide as the column up to GOAT's 640 (360 on a phone): a fixed 640
    // ran past any column narrower than that (the app reviewer and the
    // coordinator, after TRACKER 798 brought desktop widths down to 600).
    let card = div()
        .id("message-search-example")
        .occlude()
        .w_full()
        .max_w(px(if compact { 360.0 } else { 640.0 }))
        .max_h(px(480.0))
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .p(px(4.0))
        .rounded(px(12.0))
        .border_1()
        .border_color(neutral.border_accent)
        .bg(neutral.surface1)
        .shadow(popup::shadow_md(tokens))
        .face(Type::Body)
        .child(
            div()
                .px(px(8.0))
                .pt(px(6.0))
                .pb(px(4.0))
                .type_(Type::Kicker)
                .text_color(neutral.foreground_muted)
                .child(shell.t.lang.conversation().search_group()),
        )
        .children(listed)
        .children(message_search_note(shell, tokens));
    Some(
        div()
            .absolute()
            .top(px(56.0))
            .left_0()
            .right_0()
            .flex()
            .justify_center()
            .child(card)
            .into_any_element(),
    )
}
