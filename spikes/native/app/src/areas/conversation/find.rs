//! Find in a session: GOAT's thread find bar
//! (`apps/web/src/components/chat/ThreadFindBar.tsx`) with the page's
//! rules (`features/session/find-bar.tsx`, `data/find.ts`): Mod+F opens
//! it over the conversation's top right; the count sits in the field's
//! shell with previous, next and close; Enter goes to the next match,
//! Shift+Enter to the previous, Escape closes; going round an end says
//! so. What matches is the view's (`view::timeline::find`).
//!
//! [Decision Log]
//! - Purpose: find a word in a conversation of 20,000 entries without the
//!   window waiting on it.
//! - Constraint: the first search over a long log folds every entry's
//!   text (1.1 s for 20,000 in a debug build); typing must not stall.
//! - Alternatives: search all at once on every edit (the stall above);
//!   a thread of its own (the entries live in the view's state, which the
//!   UI thread owns).
//! - Choice: the view's `Finder`, a slice of 200 entries per step, driven
//!   by a task that yields between steps, so frames are drawn in between;
//!   a newer query bumps a generation and the older search stops at its
//!   next step. The folded texts are cached across searches by entry and
//!   revision (`FindCache`), so the second search is quick.
//! - Shortcut: no element of the conversation holds the keys while the
//!   composer does, so Mod+F is heard by a keystroke observer the area
//!   keeps (`observe_keystrokes`), not by an action on an element. The
//!   app's shortcuts (track S2's registry, rebindable) call
//!   [`Shell::conversation_find`]; once they bind Find, the observer goes
//!   (opening twice for one press does no harm meanwhile).
//! - Marks: the matches are marked in a person's messages, in replies
//!   and in an answered question's text (GOAT's yellow, the current one
//!   amber, `Marking`); the search runs again as the session grows while
//!   the bar is open (`find_grew`). A question still waiting is answered
//!   over the composer, so its row in the timeline is one quiet line and
//!   holds no mark.

use std::hash::{DefaultHasher, Hash as _, Hasher as _};
use std::time::Duration;

use vessel_app_kit::ui::{Context, Pixels, Window, px};
use vessel_app_kit::{Reveal, TextField, VirtualList};
use vessel_app_view::timeline::{Finder, MAX_QUERY, Match, start_at, step};
use vessel_proto::EventSeq;

use crate::shell::Shell;

mod bar;

pub(super) use bar::bar;

/// The bar's width (GOAT's `--thread-details-panel-width`, 17.5rem) and
/// its distance from the conversation's edges (`top-3 right-3`).
const WIDTH: f32 = 280.0;
const INSET: f32 = 12.0;

/// What the bar covers at the conversation's top, with a gap under it:
/// its inset, its height and 8 (on a phone it covers nothing).
const CLEAR: f32 = 56.0;

/// The pause between two slices of a search: long enough for a frame to
/// be drawn between them.
const BETWEEN_STEPS: Duration = Duration::from_millis(1);

/// The find bar, while it is open.
pub(crate) struct FindBar {
    field: TextField,
    query: String,
    /// The search under way, if one is.
    finder: Option<Finder>,
    matches: Vec<Match>,
    index: usize,
    /// What the last step said ("Wrapped to the first match."): for a
    /// screen reader, as the page says it (not drawn, as the page does
    /// not show it either); the screen-reader buffer will read it.
    pub(crate) said: Option<&'static str>,
    /// Bumped by every new search: an older one stops at its next step.
    generation: u64,
    /// Whether the search under way is a refresh of the same query.
    refreshing: bool,
    /// Whether the session grew again during a refresh.
    again: bool,
    /// The match current when a refresh began.
    keep: Option<Match>,
    /// Opened at a message (a search hit, GOAT's find "on the word at the
    /// matched message"): the record it starts at, until the search has
    /// read that far and made the first match there current.
    at: Option<EventSeq>,
}

/// What find marks on screen: its query, and the current match.
#[derive(Debug, Clone, Copy)]
pub(super) struct Marking<'a> {
    pub(super) query: &'a str,
    pub(super) current: Option<Match>,
    /// The timeline's list, which a reply scrolls to its current match.
    pub(super) list: &'a VirtualList,
    /// What the bar covers at the list's top, kept clear when scrolling.
    pub(super) clear_top: Pixels,
}

impl Marking<'_> {
    /// The marks' key for entry `entry`: the query and, when the current
    /// match is in it, which one.
    pub(super) fn key(&self, entry: usize) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.query.hash(&mut hasher);
        self.current_in(entry).hash(&mut hasher);
        hasher.finish()
    }

    /// Which match of entry `entry` is current, if one is.
    pub(super) fn current_in(&self, entry: usize) -> Option<usize> {
        self.current
            .filter(|found| found.entry == entry)
            .and_then(|found| usize::try_from(found.nth).ok())
    }
}

impl FindBar {
    /// What to mark, once a search has found its matches.
    pub(super) fn marking<'a>(
        &'a self,
        list: &'a VirtualList,
        narrow: bool,
    ) -> Option<Marking<'a>> {
        (self.finder.is_none() && !self.matches.is_empty()).then(|| Marking {
            query: &self.query,
            current: self.matches.get(self.index).copied(),
            list,
            clear_top: px(if narrow { 0.0 } else { CLEAR }),
        })
    }
}

// The portable resident boundary is deliberately limited to controllers that
// exist on this base. Full A3 adds metadata/jump votes at its consumer union.
#[cfg(test)]
impl super::ConversationArea {
    /// An open editor or pending reveal must not silently become snapshot-ready.
    pub(crate) fn navigation_held(&self) -> bool {
        self.find.is_some()
            || self.reveal.is_some()
            || self.outline_pointer.navigation_held()
            || self.origin.area().is_some_and(Self::navigation_held)
    }

    /// Retire this exact resource's work, retaining input and its real identity.
    pub(crate) fn navigation_retired(&mut self) {
        if let Some(bar) = &mut self.find {
            bar.finder = None;
            bar.generation = bar.generation.wrapping_add(1);
            bar.matches.clear();
            bar.index = 0;
            bar.said = None;
            bar.refreshing = false;
            bar.again = false;
            bar.keep = None;
            bar.at = None;
        }
        let _was_visible = self.outline_pointer.leave();
        // `reveal` also carries inbox/other owners' requests on this base.
        // Only the core's corresponding retirement may cancel that request.
        if let Some(origin) = self.origin.area_mut() {
            origin.navigation_retired();
        }
    }
}

impl std::fmt::Debug for FindBar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FindBar")
            .field("query", &self.query)
            .field("matches", &self.matches.len())
            .field("index", &self.index)
            .finish_non_exhaustive()
    }
}

impl Shell {
    /// Start hearing Mod+F, once.
    pub(super) fn hear_find(&mut self, cx: &mut Context<'_, Self>) {
        if self.areas.conversation.keys.is_some() {
            return;
        }
        let keys = cx.observe_keystrokes(|shell, event, window, cx| {
            let stroke = &event.keystroke;
            let modifiers = stroke.modifiers;
            if stroke.key == "f" && modifiers.secondary(cx) && !modifiers.shift && !modifiers.alt {
                shell.conversation_find(window, cx);
            }
        });
        self.areas.conversation.keys = Some(keys);
    }

    /// Find in the session shown (the shortcuts' Find, Mod+F by default):
    /// open the bar, or give its field the keys again; nothing when no
    /// session shows. Calling it twice for one press does no harm.
    pub(crate) fn conversation_find(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if self.areas.conversation.shown.is_none() {
            return;
        }
        self.open_find(window, cx);
    }

    /// Update a retained field's language without clearing its search.
    pub(crate) fn refresh_find_words(&self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if let Some(bar) = &self.areas.conversation.find {
            bar.field
                .set_placeholder(self.t.lang.conversation().find_in_session(), window, cx);
        }
    }

    /// Open the bar, or give its field the keys again.
    fn open_find(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        if let Some(bar) = &self.areas.conversation.find {
            bar.field.focus(window, cx);
            return;
        }
        let words = self.t.lang.conversation();
        let field = TextField::with_change(
            words.find_in_session(),
            Self::find_entered,
            Self::find_changed,
            window,
            cx,
        );
        field.focus(window, cx);
        self.areas.conversation.find = Some(FindBar {
            field,
            query: String::new(),
            finder: None,
            matches: Vec::new(),
            index: 0,
            said: None,
            generation: 0,
            refreshing: false,
            again: false,
            keep: None,
            at: None,
        });
        cx.notify();
    }

    /// Open find on `query` at the message starting at record `at` (a
    /// search hit): the current match is the first one there, once the
    /// session is read that far; until then none is brought into view.
    pub(super) fn find_at(
        &mut self,
        query: &str,
        at: EventSeq,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.open_find(window, cx);
        let Some(bar) = self.areas.conversation.find.as_mut() else {
            return;
        };
        bar.at = Some(at);
        bar.field.set_text(query, window, cx);
        self.find_changed(query.to_owned(), cx);
    }

    /// Close the bar.
    pub(super) fn close_find(&mut self, cx: &mut Context<'_, Self>) {
        if self.areas.conversation.find.take().is_some() {
            cx.notify();
        }
    }

    fn find_entered(&mut self, _text: String, _window: &mut Window, cx: &mut Context<'_, Self>) {
        self.find_move(true, cx);
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "the field's change callback hands the text over by value"
    )]
    fn find_changed(&mut self, text: String, cx: &mut Context<'_, Self>) {
        let Some(bar) = self.areas.conversation.find.as_mut() else {
            return;
        };
        let query: String = text.chars().take(MAX_QUERY).collect();
        if query == bar.query {
            return;
        }
        bar.query = query;
        bar.matches.clear();
        bar.index = 0;
        bar.said = None;
        self.start_search(false, cx);
        cx.notify();
    }

    /// Search again for the same query: the session grew while the bar
    /// is open. One search at a time while it streams: a refresh asked
    /// for during another runs once that one ends.
    pub(super) fn find_grew(&mut self, cx: &mut Context<'_, Self>) {
        let Some(bar) = self.areas.conversation.find.as_mut() else {
            return;
        };
        if bar.query.trim().is_empty() {
            return;
        }
        if bar.finder.is_some() {
            // A first search reads the log as it stands at each step and
            // takes in the new entries; a refresh under way does not.
            bar.again = bar.refreshing;
            return;
        }
        self.start_search(true, cx);
    }

    /// Start a search for the bar's query; `refresh`: the same query
    /// again, keeping the current match where it is.
    fn start_search(&mut self, refresh: bool, cx: &mut Context<'_, Self>) {
        let Some(bar) = self.areas.conversation.find.as_mut() else {
            return;
        };
        bar.finder = Some(Finder::new(&bar.query));
        bar.refreshing = refresh;
        bar.again = false;
        bar.keep = refresh
            .then(|| bar.matches.get(bar.index).copied())
            .flatten();
        bar.generation = bar.generation.wrapping_add(1);
        let generation = bar.generation;
        cx.spawn(async move |this, cx| {
            loop {
                let Ok(done) = this.update(cx, |shell, cx| shell.find_step(generation, cx)) else {
                    break;
                };
                if done {
                    break;
                }
                cx.background_executor().timer(BETWEEN_STEPS).await;
            }
        })
        .detach();
    }

    /// One slice of the search for `generation`; whether it is over (done,
    /// or overtaken by a newer query, or the bar closed).
    fn find_step(&mut self, generation: u64, cx: &mut Context<'_, Self>) -> bool {
        let area = &mut self.areas.conversation;
        let Some(bar) = area.find.as_mut() else {
            return true;
        };
        if bar.generation != generation {
            return true;
        }
        let Some(finder) = bar.finder.as_mut() else {
            return true;
        };
        let Some(mut matches) = finder.step(&mut area.find_cache, area.timeline.entries()) else {
            return false;
        };
        // What a rewind left behind is not drawn, so find skips it.
        matches.retain(|found| !area.timeline.rewound(found.entry));
        bar.finder = None;
        let start = bar
            .at
            .and_then(|at| start_at(area.timeline.entries(), area.timeline.next(), &matches, at));
        if bar.refreshing {
            // The same match stays current, found again by what it is (the
            // page's rule: a refresh must not take the reader back to it).
            let at = bar
                .keep
                .and_then(|keep| matches.iter().position(|found| *found == keep));
            bar.index = at.unwrap_or_else(|| bar.index.min(matches.len().saturating_sub(1)));
            bar.matches = matches;
            bar.refreshing = false;
            let again = std::mem::take(&mut bar.again);
            if let Some(index) = start {
                bar.index = index;
                bar.at = None;
                self.find_reveal(cx);
            }
            if again {
                self.start_search(true, cx);
            }
        } else {
            bar.matches = matches;
            bar.index = start.unwrap_or(0);
            // Opened at a message not read yet: nothing is brought into
            // view (the message is, once read); a refresh settles it.
            if bar.at.is_none() || start.is_some() {
                bar.at = None;
                self.find_reveal(cx);
            }
        }
        cx.notify();
        true
    }

    /// The next (`forward`) or previous match.
    fn find_move(&mut self, forward: bool, cx: &mut Context<'_, Self>) {
        let words = self.t.lang.conversation();
        let Some(bar) = self.areas.conversation.find.as_mut() else {
            return;
        };
        if bar.finder.is_some() || bar.matches.is_empty() {
            return;
        }
        let stepped = step(bar.matches.len(), bar.index, forward);
        bar.index = stepped.index;
        bar.said = stepped.wrapped.then(|| words.wrapped(forward));
        self.find_reveal(cx);
        cx.notify();
    }

    /// Bring the current match's entry into view, opening the run of tool
    /// calls that folds it.
    fn find_reveal(&mut self, cx: &mut Context<'_, Self>) {
        let area = &self.areas.conversation;
        let Some(found) = area
            .find
            .as_ref()
            .and_then(|bar| bar.matches.get(bar.index))
            .map(|found| found.entry)
        else {
            return;
        };
        let folded_in = area
            .folds
            .as_ref()
            .and_then(|folds| folds.run_of(found))
            .map(|run| run.lead)
            .filter(|lead| *lead != found && !area.open_runs.contains(lead));
        if let Some(lead) = folded_in {
            self.toggle_run(super::Seat::SESSION, lead, cx);
        }
        let area = &self.areas.conversation;
        if let Some(row) = area.rows.row_index(found, area.folds.as_ref(), &area.logs) {
            area.list.reveal(area.at(row), Reveal::Nearest);
        }
    }
}

#[cfg(test)]
mod language_tests;
