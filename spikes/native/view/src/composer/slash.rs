//! The composer's `/` menu: the agent's commands at the start of a prompt,
//! as Paseo's composer offers them
//! (`packages/app/src/utils/agent-command-autocomplete.ts`,
//! `hooks/use-agent-autocomplete.ts`, `hooks/use-autocomplete.ts`,
//! `components/ui/autocomplete-utils.ts`).
//!
//! What it decides: whether the caret is in a command's name, which
//! commands answer what is typed and in which order, which row is chosen
//! and how the arrows move it, and what choosing writes into the text.
//!
//! [Decision Log]
//! - Purpose: the coordinator's row b (the agent's commands and skills on
//!   `/`), at Paseo's behaviour; the commands already arrive
//!   (`commands_changed`, folded into the conversation's facts).
//! - Constraint: a command is used by sending a prompt that starts with
//!   `/name`; the node passes it to the agent as text. The node does not
//!   say which commands are skills (ACP has no such kind), and sends no
//!   commands for a chat not yet started (`harness.options` carries
//!   none).
//! - Alternatives: Paseo also completes a skill's `/name` in the middle of
//!   a prompt; the page completes only at the start and ranks by prefix,
//!   then substring, then description.
//! - Choice: Paseo's detection, ranking, order and keys, at the start of
//!   the prompt only (without a skill kind, an inline `/` would offer
//!   every command where only skills apply); a running session only.
//! - Cost: a skill cannot be completed mid-prompt; the new chat has no
//!   menu until the agent's first session reports its commands.

use vessel_proto::AgentCommand;

use super::matching::score_fields;

/// Characters that end a command's name as it is typed (Paseo's
/// `INVALID_SLASH_COMMAND_QUERY_CHARS`).
fn ends_name(character: char) -> bool {
    character == '/' || character == '"' || character == '\'' || character.is_whitespace()
}

/// The command name being typed: the text from its `/` to the caret.
/// Offsets are bytes of the text, on character boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashRange {
    /// Where the `/` is (the text's start).
    pub start: usize,
    /// The caret.
    pub end: usize,
    /// What is typed after the `/`.
    pub query: String,
}

/// The command name the caret is in, if any: the text starts with `/` and
/// nothing between it and the caret ends a name (Paseo's
/// `findActiveSlashCommand` at the prompt's start).
#[must_use]
pub fn active(text: &str, caret: usize) -> Option<SlashRange> {
    let caret = floor_boundary(text, caret);
    let typed = text.get(..caret)?.strip_prefix('/')?;
    if typed.chars().any(ends_name) {
        return None;
    }
    Some(SlashRange {
        start: 0,
        end: caret,
        query: typed.to_owned(),
    })
}

/// The greatest character boundary of `text` at or before `at`.
fn floor_boundary(text: &str, at: usize) -> usize {
    let mut at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at = at.saturating_sub(1);
    }
    at
}

/// Text after choosing a command, and the caret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    /// The new text.
    pub text: String,
    /// Where the caret goes (bytes).
    pub caret: usize,
}

/// `range` replaced by `/name` (Paseo's `applySlashCommandReplacement`):
/// what follows the caret stays; at the end of the text a space follows, so
/// the input is typed next. The caret goes after what was written.
#[must_use]
pub fn apply(text: &str, range: &SlashRange, name: &str) -> Applied {
    let start = floor_boundary(text, range.start);
    let end = floor_boundary(text, range.end).max(start);
    let before = text.get(..start).unwrap_or_default();
    let after = text.get(end..).unwrap_or_default();
    let spaced = if after.is_empty() { " " } else { "" };
    let mut written =
        String::with_capacity(text.len().saturating_add(name.len()).saturating_add(2));
    written.push_str(before);
    written.push('/');
    written.push_str(name);
    written.push_str(spaced);
    let caret = written.len();
    written.push_str(after);
    Applied {
        text: written,
        caret,
    }
}

/// The commands that answer `query`, best first (Paseo's
/// `filterAndRankCommandAutocompleteEntries`): every one in the agent's
/// order for an empty query; else those whose name matches every word of
/// it, by score, then by name.
#[must_use]
pub fn ranked<'a>(commands: &'a [AgentCommand], query: &str) -> Vec<&'a AgentCommand> {
    let query = query.trim();
    if query.is_empty() {
        return commands.iter().collect();
    }
    let mut scored: Vec<_> = commands
        .iter()
        .filter_map(|command| {
            score_fields(query, &[command.name.as_str()]).map(|score| (score, command))
        })
        .collect();
    scored.sort_by(|(one, a), (other, b)| {
        one.cmp(other).then_with(|| {
            let (a, b) = (a.name.as_str(), b.name.as_str());
            a.to_lowercase().cmp(&b.to_lowercase()).then(a.cmp(b))
        })
    });
    scored.into_iter().map(|(_, command)| command).collect()
}

/// Rows as the menu draws them over the composer: the best match nearest
/// the text, so last (Paseo's `orderAutocompleteOptions`, above the
/// input).
#[must_use]
pub fn drawn<T>(mut best_first: Vec<T>) -> Vec<T> {
    best_first.reverse();
    best_first
}

/// Which row is chosen in the menu, as Paseo's `useAutocomplete` keeps it:
/// the row nearest the text when the menu opens or what is typed changes,
/// kept while only the list's length holds; the arrows wrap.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Chosen {
    index: Option<usize>,
    query: String,
}

/// An arrow key in the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Up: the row above, the last after the first.
    Up,
    /// Down: the row below, the first after the last.
    Down,
}

impl Chosen {
    /// The row chosen among `rows` drawn rows, after what is typed became
    /// `query` (Paseo resets to the nearest row when the query changes or
    /// the choice fell off the list).
    pub fn follow(&mut self, query: &str, rows: usize) -> Option<usize> {
        let nearest = rows.checked_sub(1);
        if self.query == query {
            self.index = self.index.filter(|index| *index < rows).or(nearest);
        } else {
            query.clone_into(&mut self.query);
            self.index = nearest;
        }
        self.index
    }

    /// Move with an arrow over `rows` rows (Paseo's `getNextActiveIndex`).
    pub fn step(&mut self, step: Step, rows: usize) -> Option<usize> {
        let last = rows.checked_sub(1)?;
        self.index = Some(match (self.index.filter(|index| *index <= last), step) {
            (None, Step::Down) => 0,
            (None, Step::Up) => last,
            (Some(index), Step::Down) => {
                if index == last {
                    0
                } else {
                    index.saturating_add(1)
                }
            }
            (Some(index), Step::Up) => index.checked_sub(1).unwrap_or(last),
        });
        self.index
    }

    /// Point at row `index` (the pointer over it).
    pub fn point(&mut self, index: usize) {
        self.index = Some(index);
    }

    /// The row Enter or Tab takes: the one chosen, else the nearest.
    #[must_use]
    pub fn taken(&self, rows: usize) -> Option<usize> {
        self.index
            .filter(|index| *index < rows)
            .or_else(|| rows.checked_sub(1))
    }

    /// The menu closed: nothing chosen, as Paseo resets it.
    pub fn reset(&mut self) {
        self.index = None;
        self.query.clear();
    }
}

/// The `/` menu as it stands: the name being typed, the commands that
/// answer it in the order drawn (the best last, nearest the text), and the
/// row chosen. `None` when the caret is not in a command's name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Menu<'a> {
    /// The name being typed.
    pub range: SlashRange,
    /// The commands answering it, drawn order; none when the agent offers
    /// none (or has not said yet) or none answers.
    pub rows: Vec<&'a AgentCommand>,
    /// The row chosen.
    pub chosen: Option<usize>,
}

impl<'a> Menu<'a> {
    /// The menu for `text` with the caret at `caret`, over the agent's
    /// `commands` (`None` until it said), with the choice as `chosen` keeps
    /// it (not moved: the choice moves only on a key, [`Chosen::follow`]).
    #[must_use]
    pub fn of(
        text: &str,
        caret: usize,
        commands: Option<&'a [AgentCommand]>,
        chosen: &Chosen,
    ) -> Option<Self> {
        let range = active(text, caret)?;
        let rows = drawn(ranked(commands.unwrap_or_default(), &range.query));
        let chosen = chosen.clone().follow(&range.query, rows.len());
        Some(Self {
            range,
            rows,
            chosen,
        })
    }

    /// The command Enter or Tab takes.
    #[must_use]
    pub fn taken(&self, chosen: &Chosen) -> Option<&'a AgentCommand> {
        let mut chosen = chosen.clone();
        let _row = chosen.follow(&self.range.query, self.rows.len());
        chosen
            .taken(self.rows.len())
            .and_then(|index| self.rows.get(index).copied())
    }
}

#[cfg(test)]
mod tests;
