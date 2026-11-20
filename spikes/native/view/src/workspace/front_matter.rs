//! A Markdown file's front matter as Paseo's preview shows it, its
//! `file-panels/markdown-preview/document.ts` (pinned 1c00b365c, 0.15.4),
//! ported from the page's `data/workspace/front-matter.ts` with its cases.
//! A `---` block at the very top that reads as a YAML map becomes
//! key/value rows above the body. Anything else stays part of the body.
//!
//! [Decision Log]
//! - Purpose: F-F38's front matter table on the app, the same rows as the
//!   page for the same file (the coordinator's ruling of 2026-11-19).
//! - Constraint: Paseo reads the block with a full YAML parser (the `yaml`
//!   package, core schema, at most 100 aliases); the app had no YAML
//!   reader.
//! - Alternatives: a hand reader for `key: value` lines (lists, quotes,
//!   nested maps and anchors would differ from Paseo).
//! - Choice: yaml-rust2 0.13.0, without its default `encoding` feature
//!   (ruling A). Only this module names it. Its loader copies an anchored
//!   node at each alias, so the events are counted first: past 100 aliases,
//!   or past `MAX_NODES` nodes once aliases are expanded, the block is
//!   refused as Paseo refuses it, before anything is copied. The loader
//!   (`Parser::load` too), the emitter and a value's drop all descend by
//!   recursion, on the UI thread, so the events are pulled one by one and
//!   a block nested past `MAX_DEPTH`, counted as the loader copies it
//!   (an alias adds its anchor's levels where it stands), is refused there
//!   too (Paseo's reader throws on a deep enough one and keeps the
//!   document).
//! - Cost: one crate (and hashlink) in the app. A nested value is written
//!   back by yaml-rust2's emitter, which can quote or order a nested value
//!   differently from the page's `yaml` stringify. A block nested past 64
//!   levels stays in the body here, where Paseo's reader may still read it.

use std::collections::HashMap;

use yaml_rust2::parser::{Event, Parser};
use yaml_rust2::{Yaml, YamlEmitter, YamlLoader};

/// One row of the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontMatterRow {
    /// The key.
    pub key: String,
    /// Its value as Paseo writes it.
    pub value: String,
}

/// The table's rows and the body below them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownDocument {
    /// The front matter's rows (none when there is none).
    pub front_matter: Vec<FrontMatterRow>,
    /// The Markdown below it, or the whole file.
    pub body: String,
}

/// Paseo's `maxAliasCount`.
const MAX_ALIASES: usize = 100;

/// The most nodes a block may expand to, aliases included: a bound on
/// what the loader copies (100 aliases can still nest exponentially).
const MAX_NODES: usize = 10_000;

/// The deepest a block may nest: the loader, the emitter and a value's
/// drop recurse once per level, on the UI thread (a 1 MiB main stack on
/// Windows).
const MAX_DEPTH: usize = 64;

/// The block at the top of `source` and the rest after its closing line,
/// as Paseo's pattern finds them
/// (`/^﻿?---[ \t]*\r?\n([\s\S]*?)\r?\n---[ \t]*(?:\r?\n|$)/`).
fn split(source: &str) -> Option<(&str, &str)> {
    let text = source.strip_prefix('\u{feff}').unwrap_or(source);
    let after_open = text.strip_prefix("---")?;
    let after_open = after_open.trim_start_matches([' ', '\t']);
    let inside = after_open
        .strip_prefix("\r\n")
        .or_else(|| after_open.strip_prefix('\n'))?;
    let mut from = 0_usize;
    loop {
        let at = from.checked_add(inside.get(from..)?.find("\n---")?)?;
        let rest = inside.get(at.checked_add(4)?..)?;
        let line_rest = rest.trim_start_matches([' ', '\t']);
        let body = if line_rest.is_empty() {
            Some("")
        } else {
            line_rest
                .strip_prefix("\r\n")
                .or_else(|| line_rest.strip_prefix('\n'))
        };
        if let Some(body) = body {
            let yaml = inside.get(..at)?;
            return Some((yaml.strip_suffix('\r').unwrap_or(yaml), body));
        }
        from = at.checked_add(1)?;
    }
}

/// Counts aliases, the nodes the block expands to and how deep it nests
/// once aliases are expanded, event by event.
#[derive(Default)]
struct Budget {
    aliases: usize,
    nodes: usize,
    /// The collections open around the next event.
    open: Vec<Open>,
    /// What each anchor expands to.
    anchored: HashMap<usize, Expanded>,
    over: bool,
}

/// An open collection.
#[derive(Debug, Clone, Copy)]
struct Open {
    anchor: usize,
    /// Nodes inside it so far.
    nodes: usize,
    /// The deepest nesting inside it so far.
    depth: usize,
}

/// What a node expands to: its nodes, and the levels of collections it
/// nests (0 for a scalar). An alias is a copy of its anchor's.
#[derive(Debug, Clone, Copy)]
struct Expanded {
    nodes: usize,
    depth: usize,
}

const SCALAR: Expanded = Expanded { nodes: 1, depth: 0 };

impl Budget {
    /// `node` placed in the innermost open collection.
    fn add(&mut self, node: Expanded) {
        self.nodes = self.nodes.saturating_add(node.nodes);
        if let Some(open) = self.open.last_mut() {
            open.nodes = open.nodes.saturating_add(node.nodes);
            open.depth = open.depth.max(node.depth);
        }
        // An alias copies its anchor's levels below the ones open here.
        let depth = self.open.len().saturating_add(node.depth);
        if self.nodes > MAX_NODES || self.aliases > MAX_ALIASES || depth > MAX_DEPTH {
            self.over = true;
        }
    }

    fn close(&mut self) {
        if let Some(closed) = self.open.pop() {
            let whole = Expanded {
                nodes: closed.nodes.saturating_add(1),
                depth: closed.depth.saturating_add(1),
            };
            if closed.anchor != 0 {
                let _before = self.anchored.insert(closed.anchor, whole);
            }
            // Its own node was counted when it opened.
            if let Some(outer) = self.open.last_mut() {
                outer.nodes = outer.nodes.saturating_add(closed.nodes);
                outer.depth = outer.depth.max(whole.depth);
            }
        }
    }

    fn see(&mut self, event: &Event) {
        match *event {
            Event::Alias(anchor) => {
                self.aliases = self.aliases.saturating_add(1);
                let copied = self.anchored.get(&anchor).copied().unwrap_or(SCALAR);
                self.add(copied);
            }
            Event::Scalar(_, _, anchor, _) => {
                if anchor != 0 {
                    let _before = self.anchored.insert(anchor, SCALAR);
                }
                self.add(SCALAR);
            }
            Event::SequenceStart(anchor, _) | Event::MappingStart(anchor, _) => {
                self.add(SCALAR);
                self.open.push(Open {
                    anchor,
                    nodes: 0,
                    depth: 0,
                });
                if self.open.len() > MAX_DEPTH {
                    self.over = true;
                }
            }
            Event::SequenceEnd | Event::MappingEnd => self.close(),
            Event::Nothing
            | Event::StreamStart
            | Event::StreamEnd
            | Event::DocumentStart
            | Event::DocumentEnd => {}
        }
    }
}

/// `yaml` as Paseo's reader takes it: one document, a map, within the
/// alias and size budget.
fn parsed(yaml: &str) -> Option<yaml_rust2::yaml::Hash> {
    // Pulled one event at a time: `Parser::load` recurses once per level.
    let mut parser = Parser::new_from_str(yaml);
    let mut budget = Budget::default();
    loop {
        let (event, _mark) = parser.next_token().ok()?;
        if event == Event::StreamEnd {
            break;
        }
        budget.see(&event);
        if budget.over {
            return None;
        }
    }
    let mut documents = YamlLoader::load_from_str(yaml).ok()?;
    if documents.len() != 1 {
        return None;
    }
    match documents.pop()? {
        Yaml::Hash(map) => Some(map),
        Yaml::Real(_)
        | Yaml::Integer(_)
        | Yaml::String(_)
        | Yaml::Boolean(_)
        | Yaml::Array(_)
        | Yaml::Alias(_)
        | Yaml::Null
        | Yaml::BadValue => None,
    }
}

/// A scalar as JavaScript's `String` writes the value `yaml` gives.
fn scalar(value: &Yaml) -> Option<String> {
    match value {
        Yaml::String(text) => Some(text.clone()),
        Yaml::Integer(number) => Some(number.to_string()),
        Yaml::Boolean(truth) => Some(truth.to_string()),
        Yaml::Null => Some("null".to_owned()),
        Yaml::Real(text) => Some(real(text)),
        Yaml::Array(_) | Yaml::Hash(_) | Yaml::Alias(_) | Yaml::BadValue => None,
    }
}

/// A float as JavaScript writes a number: `1.0` as `1`, the infinities
/// and not-a-number by name.
fn real(text: &str) -> String {
    match Yaml::Real(text.to_owned()).as_f64() {
        Some(number) if number.is_nan() => "NaN".to_owned(),
        Some(number) if number.is_infinite() && number.is_sign_positive() => "Infinity".to_owned(),
        Some(number) if number.is_infinite() => "-Infinity".to_owned(),
        Some(number) => number.to_string(),
        None => text.to_owned(),
    }
}

/// A value as Paseo's table writes it: a list of scalars joined by ", ",
/// a scalar as it reads, anything else written back as YAML.
fn value(value: &Yaml) -> String {
    if let Yaml::Array(items) = value
        && let Some(words) = items.iter().map(scalar).collect::<Option<Vec<_>>>()
    {
        return words.join(", ");
    }
    if let Some(word) = scalar(value) {
        return word;
    }
    let mut written = String::new();
    if YamlEmitter::new(&mut written).dump(value).is_err() {
        return String::new();
    }
    // The emitter opens a document (`---`); the table shows the value.
    written
        .strip_prefix("---")
        .unwrap_or(&written)
        .trim()
        .to_owned()
}

/// `source` as the preview shows it.
#[must_use]
pub fn markdown_document(source: &str) -> MarkdownDocument {
    let whole = || MarkdownDocument {
        front_matter: Vec::new(),
        body: source.to_owned(),
    };
    let Some((yaml, body)) = split(source) else {
        return whole();
    };
    let Some(map) = parsed(yaml) else {
        return whole();
    };
    MarkdownDocument {
        front_matter: map
            .iter()
            .map(|(key, item)| FrontMatterRow {
                key: scalar(key).unwrap_or_else(|| value(key)),
                value: value(item),
            })
            .collect(),
        body: body.to_owned(),
    }
}

#[cfg(test)]
mod tests;
