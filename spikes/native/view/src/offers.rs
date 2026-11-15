//! What an agent on a machine offers (its options: models, efforts,
//! permissions modes, on/off options; and its `/` commands, as its last
//! session reported them), kept so a control or the `/` menu opens on the
//! last list at once and refreshed behind it (owner word 28: "keep it, refresh
//! it in the background, and show it the moment it is clicked").
//!
//! [Decision Log]
//! - Purpose: opening the model, effort or permissions control showed a
//!   spinner until the node answered `harness.options`, which can take
//!   seconds (the node starts the agent to ask it when its own ten-minute
//!   copy is gone), and again after every restart.
//! - Constraint: the lists are the agent's, never Vessel's (ADR 0022): a
//!   kept list is only ever the agent's last answer, shown until a newer
//!   one; nothing is invented when there is none. A refresh must not move
//!   the row under the pointer or lose the person's choice.
//! - Followed: Paseo's provider snapshot. It keeps the last snapshot per
//!   host and folder on disk (`data/provider-snapshot-cache.ts`, versioned,
//!   size-bounded), hands it to the query as its first data, and refetches
//!   when a selector opens (`hooks/use-providers-snapshot.ts`,
//!   `selectorOpenRefetchDecision`: always when the selected provider has
//!   no entry yet, else when stale); models are shown whenever any exist,
//!   loading or the error only when none (`provider-selection.ts`,
//!   `buildEntryModelSelection`); a remembered choice the agent stopped
//!   offering falls back to what is shown (`resolveEffectiveComposerModeId`;
//!   here the agent's own value, by `with_picks`).
//! - Alternatives: keep asking on every open, as before (the spinner the
//!   owner reported); refresh on a timer (asks agents no one is looking
//!   at, and starts them on the node); replace the shown list wholesale on
//!   an answer, as Paseo's query does (Paseo's virtualised list keys rows
//!   by id; the kit's list highlights by index, so a row added above would
//!   move the highlight and the row under the pointer).
//! - Choice: one store per machine and agent here, without a clock of its
//!   own (`now` is passed in) and without I/O; the link keeps the copy on
//!   disk and hands it over at connect (`link::offers`). An answer that
//!   arrives while a list is open is laid over the shown one ([`arrange`]):
//!   what stays keeps its place, what is new goes at the edge away from
//!   the one the list is anchored by (the composer's lists open above
//!   their control, so at the top: the rows under the pointer stay where
//!   they are on screen), next to its group when it has one; what is gone
//!   leaves.
//! - Trade-offs: a closed list takes the agent's order as sent; an open
//!   one may show a new value out of the agent's order until it is opened
//!   again. A value removed, or one added to a group, still moves the rows
//!   on its far side by one row.
//!   Requests are bounded by [`OPEN_GAP`], [`RETRY_GAP`] and one request
//!   in flight per agent: ten opens are one request.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use vessel_proto::{AgentCommand, HarnessRef, OptionKind, OptionValue, SessionOption};

/// After a good answer, opening a control does not ask again for this
/// long (the node's own copy lasts ten minutes, so a sooner ask only reads
/// it back; the bound keeps a burst of opens to one request).
pub const OPEN_GAP: Duration = Duration::from_secs(30);

/// After a failed answer, opening a control asks again only after this
/// long (an offline machine answers at once, so without it every open
/// would be a request).
pub const RETRY_GAP: Duration = Duration::from_secs(10);

/// A request unanswered for this long no longer holds back the next one
/// (the link always answers, but a call can hang on a dying connection).
pub const ASK_TIMEOUT: Duration = Duration::from_secs(60);

/// Whose list: an agent on a machine (by the machine's name in the book;
/// the link keys its file by the node's id, so a name given to another
/// node never shows this one's list).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OfferKey {
    /// The machine's name.
    pub machine: String,
    /// The agent.
    pub agent: HarnessRef,
}

impl OfferKey {
    /// The key of `agent` on `machine`.
    #[must_use]
    pub fn new(machine: impl Into<String>, agent: HarnessRef) -> Self {
        Self {
            machine: machine.into(),
            agent,
        }
    }
}

/// Why a refresh is wanted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// A control opened, or the draft settled on the agent.
    Open,
    /// The machine connected (again).
    Connect,
    /// The machine's agents changed (one installed, removed, updated).
    AgentsChanged,
    /// The person asked again (Settings › Agents' retry): asks unless a
    /// request is in flight, however recent the last answer.
    Retry,
}

/// How current a shown list is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    /// The agent's answer this run.
    Fresh,
    /// Kept from an earlier run, not refreshed yet.
    Kept,
    /// The last refresh failed; this is the earlier list.
    Stale,
}

/// What a control shows for an agent (Paseo's `ProviderModelSelection`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Offered<'a> {
    /// A list, shown at once whenever there is one.
    Ready {
        /// The options, in the order shown.
        options: &'a [SessionOption],
        /// How current it is ([`Freshness::Stale`] is said once, quietly).
        freshness: Freshness,
    },
    /// No list yet, and one is coming (the first time only).
    Loading,
    /// No list, and the last ask failed: why.
    Failed(&'a str),
}

/// Which edge of an open list stays where it is on screen: the one it is
/// anchored by. A list grows away from it, so a new row is laid at the
/// other edge to leave every shown row in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchored {
    /// Its top (a list opening below its control): new rows at the end.
    Top,
    /// Its bottom (a list opening above its control, a sheet from the
    /// bottom): new rows at the start.
    Bottom,
}

/// What an answer did to the store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrival {
    /// The list shown changed (or appeared): draw again.
    Changed,
    /// The same list, now confirmed fresh.
    Same,
    /// The refresh failed; the earlier list stays, marked stale.
    KeptStale,
    /// The first ask failed: nothing to show but why.
    Failed,
}

#[derive(Debug, Clone, Default)]
struct Entry {
    /// The list shown, and how current it is.
    copy: Option<(Vec<SessionOption>, Freshness)>,
    /// Why the last ask failed, while there is no list.
    failed: Option<String>,
    /// When the request in flight was sent.
    asked: Option<Instant>,
    /// When the last answer came, and whether it was a list.
    answered: Option<(Instant, bool)>,
    /// The agent's `/` commands as a session of it last reported them
    /// (`harness.options` carries none), or as kept from an earlier run.
    commands: Option<Vec<AgentCommand>>,
}

/// Every agent's list the app knows, by machine and agent.
#[derive(Debug, Clone, Default)]
pub struct Offers {
    entries: HashMap<OfferKey, Entry>,
}

impl Offers {
    /// What a control for `key` shows now. An agent never asked about is
    /// [`Offered::Loading`]: the caller asks in the same frame.
    #[must_use]
    pub fn offered(&self, key: &OfferKey) -> Offered<'_> {
        let Some(entry) = self.entries.get(key) else {
            return Offered::Loading;
        };
        match (&entry.copy, &entry.failed) {
            (Some((options, freshness)), _) => Offered::Ready {
                options,
                freshness: *freshness,
            },
            (None, Some(why)) if entry.asked.is_none() => Offered::Failed(why),
            (None, _) => Offered::Loading,
        }
    }

    /// The list for `key`, however current, if there is one.
    #[must_use]
    pub fn options(&self, key: &OfferKey) -> Option<&[SessionOption]> {
        self.entries
            .get(key)
            .and_then(|entry| entry.copy.as_ref())
            .map(|(options, _)| options.as_slice())
    }

    /// Whether the store has anything for `key`: a list, a request or an
    /// answer of this run (a draft settling on an agent asks only when it
    /// has nothing; a control opening asks by [`Offers::should_ask`]).
    #[must_use]
    pub fn known(&self, key: &OfferKey) -> bool {
        self.entries.contains_key(key)
    }

    /// Whether a request for `key` is in flight at `now` (a retry is not
    /// offered meanwhile).
    #[must_use]
    pub fn asking(&self, key: &OfferKey, now: Instant) -> bool {
        self.entries
            .get(key)
            .and_then(|entry| entry.asked)
            .is_some_and(|asked| now.saturating_duration_since(asked) < ASK_TIMEOUT)
    }

    /// Whether to ask the agent now, for `why`, at `now`: never while a
    /// request is in flight; on an open, not within [`OPEN_GAP`] of a good
    /// answer nor [`RETRY_GAP`] of a failed one; a connection, a change
    /// of the machine's agents and a person's retry always ask.
    #[must_use]
    pub fn should_ask(&self, key: &OfferKey, why: Why, now: Instant) -> bool {
        let Some(entry) = self.entries.get(key) else {
            return true;
        };
        if self.asking(key, now) {
            return false;
        }
        if why != Why::Open {
            return true;
        }
        match entry.answered {
            Some((at, true)) => now.saturating_duration_since(at) >= OPEN_GAP,
            Some((at, false)) => now.saturating_duration_since(at) >= RETRY_GAP,
            None => true,
        }
    }

    /// Note a request for `key` sent at `now`.
    pub fn asked(&mut self, key: OfferKey, now: Instant) {
        self.entries.entry(key).or_default().asked = Some(now);
    }

    /// [`Offers::should_ask`], and when it says so, [`Offers::asked`]:
    /// `true` when the caller is to send the request.
    pub fn want(&mut self, key: &OfferKey, why: Why, now: Instant) -> bool {
        let ask = self.should_ask(key, why, now);
        if ask {
            self.asked(key.clone(), now);
        }
        ask
    }

    /// What was kept from an earlier run, handed over at connect: the
    /// options, the commands, or both. Each is taken only when there is
    /// none yet: what this run heard is newer.
    pub fn kept(
        &mut self,
        key: OfferKey,
        options: Option<Vec<SessionOption>>,
        commands: Option<Vec<AgentCommand>>,
    ) -> Arrival {
        let entry = self.entries.entry(key).or_default();
        let mut arrival = Arrival::Same;
        if let Some(options) = options
            && entry.copy.is_none()
        {
            entry.copy = Some((options, Freshness::Kept));
            entry.failed = None;
            arrival = Arrival::Changed;
        }
        if let Some(commands) = commands
            && entry.commands.is_none()
        {
            entry.commands = Some(commands);
            arrival = Arrival::Changed;
        }
        arrival
    }

    /// The `/` commands of `key`'s agent as last known, if any.
    #[must_use]
    pub fn commands(&self, key: &OfferKey) -> Option<&[AgentCommand]> {
        self.entries
            .get(key)
            .and_then(|entry| entry.commands.as_deref())
    }

    /// A session of `key`'s agent reported its commands: `true` when they
    /// differ from what was known (then they are to be kept on the
    /// device too).
    pub fn commands_reported(&mut self, key: OfferKey, commands: &[AgentCommand]) -> bool {
        let entry = self.entries.entry(key).or_default();
        if entry.commands.as_deref() == Some(commands) {
            return false;
        }
        entry.commands = Some(commands.to_vec());
        true
    }

    /// The agent's answer for `key` at `now`. `open`: a list showing it is
    /// open, anchored by that edge, so a new list is laid over the shown
    /// one ([`arrange`]) rather than replacing it. A failure keeps a list
    /// there is, marked stale.
    pub fn answered(
        &mut self,
        key: OfferKey,
        outcome: Result<Vec<SessionOption>, String>,
        open: Option<Anchored>,
        now: Instant,
    ) -> Arrival {
        let entry = self.entries.entry(key).or_default();
        entry.asked = None;
        entry.answered = Some((now, outcome.is_ok()));
        match outcome {
            Ok(fresh) => {
                entry.failed = None;
                let (options, changed) = match entry.copy.take() {
                    Some((shown, _)) => {
                        let options = match open {
                            Some(anchored) => arrange(fresh, &shown, anchored),
                            None => fresh,
                        };
                        let changed = options != shown;
                        (options, changed)
                    }
                    None => (fresh, true),
                };
                entry.copy = Some((options, Freshness::Fresh));
                if changed {
                    Arrival::Changed
                } else {
                    Arrival::Same
                }
            }
            Err(why) => {
                if let Some((_, freshness)) = &mut entry.copy {
                    *freshness = Freshness::Stale;
                    Arrival::KeptStale
                } else {
                    entry.failed = Some(why);
                    Arrival::Failed
                }
            }
        }
    }

    /// The agents of `machine` the store has a list for (asked again when
    /// the machine connects).
    #[must_use]
    pub fn agents_on(&self, machine: &str) -> Vec<HarnessRef> {
        let mut agents: Vec<HarnessRef> = self
            .entries
            .iter()
            .filter(|(key, entry)| {
                key.machine == machine && (entry.copy.is_some() || entry.commands.is_some())
            })
            .map(|(key, _)| key.agent.clone())
            .collect();
        agents.sort_by_key(|agent| format!("{agent:?}"));
        agents
    }

    /// The machine `from` is called `to` now: its lists follow the name.
    pub fn renamed(&mut self, from: &str, to: &str) {
        self.entries = std::mem::take(&mut self.entries)
            .into_iter()
            .map(|(mut key, entry)| {
                if key.machine == from {
                    to.clone_into(&mut key.machine);
                }
                (key, entry)
            })
            .collect();
    }

    /// The machine `name` is forgotten: its lists go.
    pub fn forgotten(&mut self, name: &str) {
        self.entries.retain(|key, _| key.machine != name);
    }
}

/// A fresh answer laid over the list shown, so nothing under the pointer
/// moves: the options and values that stay keep their places, in the
/// shown order; new ones go at the edge away from the list's anchor, in
/// the agent's order (a new value of a group already shown goes next to
/// it, on that same side, so a group's values stay together); gone ones
/// leave. Everything else (names, descriptions, the value in effect, a
/// changed kind) is the fresh answer's.
#[must_use]
pub fn arrange(
    fresh: Vec<SessionOption>,
    shown: &[SessionOption],
    anchored: Anchored,
) -> Vec<SessionOption> {
    let mut pool: Vec<Option<SessionOption>> = fresh.into_iter().map(Some).collect();
    let mut kept = Vec::with_capacity(pool.len());
    for old in shown {
        if let Some(new) = take_where(&mut pool, |new| new.id == old.id) {
            kept.push(arrange_values(new, old, anchored));
        }
    }
    let added: Vec<SessionOption> = pool.into_iter().flatten().collect();
    match anchored {
        Anchored::Top => {
            kept.extend(added);
            kept
        }
        Anchored::Bottom => {
            let mut arranged = added;
            arranged.extend(kept);
            arranged
        }
    }
}

/// An option's values laid over the shown ones ([`arrange`]).
fn arrange_values(
    mut fresh: SessionOption,
    shown: &SessionOption,
    anchored: Anchored,
) -> SessionOption {
    if let (OptionKind::Select { values, .. }, OptionKind::Select { values: old, .. }) =
        (&mut fresh.kind, &shown.kind)
    {
        let mut pool: Vec<Option<OptionValue>> =
            std::mem::take(values).into_iter().map(Some).collect();
        let mut ordered: Vec<OptionValue> = Vec::with_capacity(pool.len());
        for value in old {
            if let Some(kept) = take_where(&mut pool, |new| new.id == value.id) {
                ordered.push(kept);
            }
        }
        let added: Vec<OptionValue> = pool.into_iter().flatten().collect();
        // The new values at the start keep the agent's order: each is laid
        // after the new ones before it (`at_start` counts them).
        let mut at_start = 0_usize;
        for new in added {
            let same = |value: &OptionValue| new.group.is_some() && value.group == new.group;
            let after_last = |ordered: &[OptionValue]| {
                ordered
                    .iter()
                    .rposition(same)
                    .map(|last| last.saturating_add(1))
            };
            let (place, in_start) = match anchored {
                Anchored::Top => (after_last(&ordered).unwrap_or(ordered.len()), false),
                Anchored::Bottom => match ordered.iter().position(same) {
                    // A group the new values began: after its last one.
                    Some(first) if first < at_start => {
                        (after_last(&ordered).unwrap_or(at_start), true)
                    }
                    // A group shown already: before its first value.
                    Some(first) => (first, false),
                    None => (at_start, true),
                },
            };
            ordered.insert(place, new);
            if in_start {
                at_start = at_start.saturating_add(1);
            }
        }
        *values = ordered;
    }
    fresh
}

/// The first item of `pool` that `matches`, taken out of it.
fn take_where<T>(pool: &mut [Option<T>], matches: impl Fn(&T) -> bool) -> Option<T> {
    pool.iter_mut()
        .find(|slot| slot.as_ref().is_some_and(&matches))
        .and_then(Option::take)
}

#[cfg(test)]
mod tests;
