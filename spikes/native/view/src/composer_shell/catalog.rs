//! One ordered command catalog merges ownership before applying autocomplete filters.
//! Plugin identity stays generic so the runtime's canonical contribution address is
//! carried intact rather than replaced by another local string identifier.

use std::collections::HashSet;

use vessel_proto::{AgentCommand, conversation::CommandName};

use crate::composer::matching::score_fields;
use crate::composer::slash::SlashRange;

use super::commands::{ClientCommand, Context};

/// A provider command's known inline eligibility, projected from wire metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inline {
    /// Only an explicitly classified provider skill is eligible.
    Skill,
    /// A command, unknown kind or old peer remains root-only.
    RootOnly,
}

/// Built-ins stay reserved for collision checks even when hidden in a draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// Existing session: show and execute built-ins.
    Session,
    /// New-session draft: hide built-ins.
    Draft,
}

/// Slash position controls which kinds of suggestions are meaningful.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    /// Client, plugin and provider commands are offered at prompt start.
    Start,
    /// Only provider skills are offered after whitespace.
    Inline,
}

/// A parsed slash range preserves UTF-8 byte positions for the existing editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Active {
    /// Existing replacement helper consumes this range.
    pub range: SlashRange,
    /// Inline skills never become root-only command suggestions.
    pub position: Position,
}

/// A command is owned by exactly one execution path.
#[derive(Debug, PartialEq, Eq)]
pub enum Source<'a, Plugin> {
    /// The lifecycle dispatcher owns these operations.
    Client(ClientCommand),
    /// The canonical plugin address survives selection and invocation.
    Plugin(&'a Plugin),
    /// Provider commands are sent as normal prompt text.
    Provider {
        /// The provider's unchanged descriptor and argument hint.
        command: &'a AgentCommand,
        /// Only explicitly classified skills can complete inline.
        inline: Inline,
    },
}

/// A validated command name, or a compile-time built-in enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Name<'a> {
    Client(ClientCommand),
    Offered(&'a CommandName),
}

impl<'a> Name<'a> {
    fn text(self) -> &'a str {
        match self {
            Self::Client(command) => command.name(),
            Self::Offered(name) => name.as_str(),
        }
    }
}

/// Catalog metadata is borrowed, preserving descriptions and runtime identities.
#[derive(Debug, PartialEq, Eq)]
pub struct Entry<'a, Plugin> {
    name: Name<'a>,
    /// The renderer/dispatcher matches ownership instead of matching name strings.
    pub source: Source<'a, Plugin>,
}

impl<'a, Plugin> Entry<'a, Plugin> {
    /// Canonical plugin contributions are adapted at the boundary once.
    #[must_use]
    pub const fn plugin(name: &'a CommandName, plugin: &'a Plugin) -> Self {
        Self {
            name: Name::Offered(name),
            source: Source::Plugin(plugin),
        }
    }
    /// The link projects optional provider kind to known inline eligibility.
    #[must_use]
    pub const fn provider(command: &'a AgentCommand, inline: Inline) -> Self {
        Self {
            name: Name::Offered(&command.name),
            source: Source::Provider { command, inline },
        }
    }
    /// Display/replacement text shares the exact name used in collision checks.
    #[must_use]
    pub fn name(&self) -> &str {
        self.name.text()
    }

    fn fields(&self) -> Vec<&str> {
        let mut fields = vec![self.name()];
        if let Source::Client(command) = self.source {
            fields.extend_from_slice(command.aliases());
        }
        fields
    }
}

/// Built-ins and aliases win, then the first plugin, then unclaimed provider names.
/// Draft hiding occurs after reservation, matching the reference merge/filter order.
#[must_use]
pub fn merge<'a, Plugin>(
    target: Target,
    plugins: impl IntoIterator<Item = Entry<'a, Plugin>>,
    providers: impl IntoIterator<Item = Entry<'a, Plugin>>,
) -> Vec<Entry<'a, Plugin>> {
    let mut claimed = HashSet::new();
    let mut entries = Vec::new();
    for command in ClientCommand::ALL {
        let _new = claimed.insert(command.name());
        for alias in command.aliases() {
            let _new = claimed.insert(*alias);
        }
        if target == Target::Session {
            entries.push(Entry {
                name: Name::Client(command),
                source: Source::Client(command),
            });
        }
    }
    for plugin in plugins {
        if claimed.insert(plugin.name.text()) {
            entries.push(plugin);
        }
    }
    for provider in providers {
        if !claimed.contains(provider.name.text()) {
            entries.push(provider);
        }
    }
    entries
}

/// Rank names and built-in aliases; descriptions never alter reference ranking.
#[must_use]
pub fn ranked<'a, 'entry, Plugin>(
    entries: &'entry [Entry<'a, Plugin>],
    query: &str,
    position: Position,
) -> Vec<&'entry Entry<'a, Plugin>> {
    let eligible = |entry: &&Entry<'a, Plugin>| {
        position == Position::Start
            || matches!(
                entry.source,
                Source::Provider {
                    inline: Inline::Skill,
                    ..
                }
            )
    };
    let query = query.trim_matches(crate::matcher::whitespace);
    if query.is_empty() {
        return entries.iter().filter(eligible).collect();
    }
    let mut scored: Vec<_> = entries
        .iter()
        .filter(eligible)
        .filter_map(|entry| score_fields(query, &entry.fields()).map(|score| (score, entry)))
        .collect();
    scored.sort_by(|(one, a), (other, b)| {
        one.cmp(other)
            .then_with(|| crate::alphabetical::compare(a.name(), b.name()))
    });
    scored.into_iter().map(|(_, entry)| entry).collect()
}

/// Find the nearest whitespace-delimited slash before a UTF-8-safe caret.
#[must_use]
pub fn active(text: &str, caret: usize) -> Option<Active> {
    let mut caret = caret.min(text.len());
    while !text.is_char_boundary(caret) {
        caret = caret.saturating_sub(1);
    }
    let before = text.get(..caret)?;
    for (start, _) in before.rmatch_indices('/') {
        let previous = text.get(..start)?.chars().next_back();
        if previous.is_some_and(|character| !crate::matcher::whitespace(character)) {
            continue;
        }
        let query = text.get(start.saturating_add(1)..caret)?;
        if query.chars().any(|character| {
            crate::matcher::whitespace(character) || matches!(character, '/' | '"' | '\'')
        }) {
            continue;
        }
        return Some(Active {
            range: SlashRange {
                start,
                end: caret,
                query: query.to_owned(),
            },
            position: if start == 0 {
                Position::Start
            } else {
                Position::Inline
            },
        });
    }
    None
}

/// Arguments remain prompt text, trimmed once before plugin invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Arguments<'a>(&'a str);
impl Arguments<'_> {
    /// The plugin receives this exact text, without repeated whitespace parsing.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0
    }
}

/// Parse a plugin invocation only after built-in dispatch declined the whole prompt.
#[must_use]
pub fn plugin_call<'a, 'entry, Plugin>(
    text: &'a str,
    context: Context,
    entries: &'entry [Entry<'_, Plugin>],
) -> Option<(&'entry Plugin, Arguments<'a>)> {
    if context == Context::Attached {
        return None;
    }
    let token = text
        .trim_matches(crate::matcher::whitespace)
        .strip_prefix('/')?;
    let (name, args) = token
        .find(crate::matcher::whitespace)
        .map_or((token, ""), |at| {
            (
                token.get(..at).unwrap_or_default(),
                token
                    .get(at..)
                    .unwrap_or_default()
                    .trim_matches(crate::matcher::whitespace),
            )
        });
    entries.iter().find_map(|entry| match entry.source {
        Source::Plugin(plugin) if entry.name() == name => Some((plugin, Arguments(args))),
        Source::Client(_) | Source::Provider { .. } | Source::Plugin(_) => None,
    })
}

#[cfg(test)]
mod tests;
