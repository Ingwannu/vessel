//! Claude Code's `settings.json` and Codex's `hooks.json`: a `hooks` object
//! with one list of matcher entries per event (Paseo's
//! `claude/claude-settings.ts` and `codex/codex-settings.ts`).
//!
//! [Decision Log]
//! - Purpose: add Vessel's entry to each event, or take it out, and change
//!   nothing else in a file that is a person's own.
//! - Paseo parses the whole file, replaces its marked entries, and writes
//!   it back with `JSON.stringify(config, null, 2)`. Text that is not JSON
//!   stops it with an error (`claude-settings.ts:23-24`); JSON that is not
//!   an object becomes `{}` (`:25-27`).
//! - Choice:
//!   - The file is read as an object that keeps its keys in order and each
//!     value's text as written ([`Object`], on `serde_json`'s `RawValue`).
//!     It is written back as [`Out`]: every value Vessel did not change is
//!     its text as read, and only the `hooks` object and the lists of
//!     Vessel's own events that did change are laid out again, in the
//!     two-space form Paseo and the agents write. `serde_json`'s map would
//!     sort the keys of the whole file, and re-printing it would re-indent a
//!     person's values.
//!   - So in a file Paseo wrote, every Paseo entry and every other value
//!     keeps its bytes and its place, and turning the setting off after
//!     installing gives back the file byte for byte (tested against Paseo
//!     0.15.4's own installer output, `tests/fixtures/agent_hooks`).
//!   - Only entries whose commands are exactly Vessel's change. While on, an
//!     entry that is already exactly Vessel's stays where it is, so Paseo
//!     reinstalling its own entries after Vessel's moves nothing back and
//!     forth.
//!   - A file that is not JSON, not an object, has `hooks` or one of
//!     Vessel's events twice, or whose `hooks` or event list has another
//!     shape is left untouched ([`Outcome::Invalid`]). Paseo would replace
//!     a non-object with `{}`.
//!   - A file that does not exist is created only to install, never to
//!     remove.
//!   - The entry is Paseo's: `{"matcher": "", "hooks": [{"type": "command",
//!     "command": ..., "timeout": 10}]}`, and Codex's also has
//!     `commandWindows`.

use std::path::Path;

use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::value::RawValue;

use super::files::{self, Outcome};
use super::{Agent, marked, shell_command, windows_command};

/// The file an agent's hooks are in, under its home.
const fn file(agent: Agent) -> &'static str {
    match agent {
        Agent::Claude => "settings.json",
        Agent::Codex | Agent::OpenCode => "hooks.json",
    }
}

/// Install (`on`) or remove `agent`'s hooks in its file under `home`.
pub(super) fn apply(agent: Agent, home: &Path, on: bool) -> Outcome {
    let path = home.join(file(agent));
    let current = match std::fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Outcome::Failed(path, error),
    };
    if current.is_none() && !on {
        return Outcome::Unchanged;
    }
    let Some(next) = changed(agent, current.as_deref(), on) else {
        return Outcome::Invalid(path);
    };
    let same = current.as_deref().is_some_and(|text| {
        serde_json::from_str::<serde_json::Value>(text).ok()
            == serde_json::from_str::<serde_json::Value>(&next).ok()
    });
    if same {
        return Outcome::Unchanged;
    }
    files::replace(&path, next.as_bytes())
}

/// The file's next text, or `None` when its shape is not one Vessel reads.
pub(super) fn changed(agent: Agent, current: Option<&str>, on: bool) -> Option<String> {
    let config = match current {
        Some(text) => serde_json::from_str::<Object>(text).ok()?,
        None => Object::default(),
    };
    if config.count("hooks") > 1 {
        return None;
    }
    let mut top = Vec::with_capacity(config.0.len().saturating_add(1));
    let mut found = false;
    for (key, value) in config.0 {
        let out = if key == "hooks" {
            found = true;
            hooks(agent, Some(value), on)?
        } else {
            Out::Kept(value)
        };
        top.push((key, out));
    }
    if !found {
        let added = hooks(agent, None, on)?;
        if !matches!(&added, Out::Map(events) if events.is_empty()) {
            top.push(("hooks".to_owned(), added));
        }
    }
    let mut text = serde_json::to_string_pretty(&Out::Map(top)).ok()?;
    text.push('\n');
    Some(text)
}

/// The `hooks` object after the change: as read when none of Vessel's
/// event lists changed. `None` when its shape is not one Vessel reads.
fn hooks(agent: Agent, value: Option<Box<RawValue>>, on: bool) -> Option<Out> {
    let events = match &value {
        Some(raw) => serde_json::from_str::<Object>(raw.get()).ok()?,
        None => Object::default(),
    };
    if agent.event_names().any(|event| events.count(event) > 1) {
        return None;
    }
    let mut out = Vec::with_capacity(events.0.len().saturating_add(5));
    let mut changed = false;
    let mut listed_events = Vec::new();
    for (name, list) in events.0 {
        let Some(event) = agent.event_names().find(|event| *event == name) else {
            out.push((name, Out::Kept(list)));
            continue;
        };
        listed_events.push(event);
        let next = event_list(agent, event, Some(list), on)?;
        if next.changed {
            changed = true;
        }
        if let Some(list) = next.out {
            out.push((name, list));
        }
    }
    for event in agent
        .event_names()
        .filter(|event| !listed_events.contains(event))
    {
        let next = event_list(agent, event, None, on)?;
        if next.changed {
            changed = true;
        }
        if let Some(list) = next.out {
            out.push((event.to_owned(), list));
        }
    }
    match value {
        Some(raw) if !changed => Some(Out::Kept(raw)),
        _ => Some(Out::Map(out)),
    }
}

/// One event's list after the change.
struct Listed {
    /// `None`: the list ends up empty, and its key goes.
    out: Option<Out>,
    /// Whether it differs from the list as read.
    changed: bool,
}

/// `event`'s list without Vessel's commands, and with Vessel's entry when
/// `on`. An entry already exactly Vessel's stays where it is, once.
fn event_list(agent: Agent, event: &str, raw: Option<Box<RawValue>>, on: bool) -> Option<Listed> {
    let listed = match &raw {
        Some(raw) => serde_json::from_str::<Vec<Box<RawValue>>>(raw.get()).ok()?,
        None => Vec::new(),
    };
    let wanted = serde_json::to_value(entry(agent, event)).ok()?;
    let mut entries = Vec::with_capacity(listed.len().saturating_add(1));
    let mut changed = false;
    let mut installed = false;
    for item in listed {
        match unmarked(agent, item) {
            Kept::Theirs(item) => entries.push(Out::Kept(item)),
            Kept::Shared(rest) => {
                changed = true;
                entries.push(rest);
            }
            Kept::Ours(item) => {
                let current = serde_json::from_str::<serde_json::Value>(item.get()).ok();
                if on && !installed && current.as_ref() == Some(&wanted) {
                    installed = true;
                    entries.push(Out::Kept(item));
                } else {
                    changed = true;
                }
            }
        }
    }
    if on && !installed {
        changed = true;
        entries.push(Out::Vessel(entry(agent, event)));
    }
    if !changed {
        return Some(Listed {
            out: raw.map(Out::Kept),
            changed,
        });
    }
    Some(Listed {
        out: (!entries.is_empty()).then_some(Out::List(entries)),
        changed,
    })
}

/// What one listed entry is, by its commands.
enum Kept {
    /// None of its commands is Vessel's (or it is not an entry Vessel
    /// reads): written back as it was.
    Theirs(Box<RawValue>),
    /// Every command is Vessel's.
    Ours(Box<RawValue>),
    /// A person's commands beside Vessel's: the entry without Vessel's, its
    /// other keys and commands as they were.
    Shared(Out),
}

fn unmarked(agent: Agent, entry: Box<RawValue>) -> Kept {
    let Ok(fields) = serde_json::from_str::<Object>(entry.get()) else {
        return Kept::Theirs(entry);
    };
    let Some(Ok(commands)) = fields
        .get("hooks")
        .map(|raw| serde_json::from_str::<Vec<Box<RawValue>>>(raw.get()))
    else {
        return Kept::Theirs(entry);
    };
    let (vessel, theirs): (Vec<_>, Vec<_>) = commands
        .into_iter()
        .partition(|command| ours(agent, command));
    if vessel.is_empty() {
        return Kept::Theirs(entry);
    }
    if theirs.is_empty() {
        return Kept::Ours(entry);
    }
    let mut rest = Some(Out::List(theirs.into_iter().map(Out::Kept).collect()));
    Kept::Shared(Out::Map(
        fields
            .0
            .into_iter()
            .map(|(key, value)| {
                let out = match rest.take() {
                    Some(list) if key == "hooks" => list,
                    other => {
                        rest = other;
                        Out::Kept(value)
                    }
                };
                (key, out)
            })
            .collect(),
    ))
}

/// Whether a hook command is exactly one Vessel installs for `agent`.
fn ours(agent: Agent, command: &RawValue) -> bool {
    let Ok(command) = serde_json::from_str::<serde_json::Value>(command.get()) else {
        return false;
    };
    ["command", "commandWindows"].into_iter().any(|key| {
        command
            .get(key)
            .and_then(serde_json::Value::as_str)
            .is_some_and(|text| marked(text, agent))
    })
}

/// Vessel's entry for one event, its keys in Paseo's order.
fn entry(agent: Agent, event: &str) -> Entry {
    Entry {
        matcher: "",
        hooks: [Hook {
            kind: "command",
            command: shell_command(agent, event),
            command_windows: (agent == Agent::Codex).then(|| windows_command(agent, event)),
            timeout: 10,
        }],
    }
}

#[derive(Serialize)]
struct Entry {
    matcher: &'static str,
    hooks: [Hook; 1],
}

#[derive(Serialize)]
struct Hook {
    #[serde(rename = "type")]
    kind: &'static str,
    command: String,
    #[serde(rename = "commandWindows", skip_serializing_if = "Option::is_none")]
    command_windows: Option<String>,
    timeout: u32,
}

/// A JSON object as written: its keys in order, each value's text as it was.
#[derive(Debug, Default)]
pub(super) struct Object(pub(super) Vec<(String, Box<RawValue>)>);

impl Object {
    fn get(&self, key: &str) -> Option<&RawValue> {
        self.0
            .iter()
            .find(|(name, _value)| name == key)
            .map(|(_name, value)| value.as_ref())
    }

    fn count(&self, key: &str) -> usize {
        self.0.iter().filter(|(name, _value)| name == key).count()
    }
}

impl<'de> Deserialize<'de> for Object {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Entries;
        impl<'de> Visitor<'de> for Entries {
            type Value = Object;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a JSON object")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Object, M::Error> {
                let mut entries = Vec::new();
                while let Some(entry) = map.next_entry::<String, Box<RawValue>>()? {
                    entries.push(entry);
                }
                Ok(Object(entries))
            }
        }
        deserializer.deserialize_map(Entries)
    }
}

/// The file as it is written back. `serde_json`'s pretty writer puts a
/// [`Out::Kept`] value's text in verbatim, so a value Vessel did not change
/// keeps its bytes; at the depth it was read from, in a two-space file, its
/// indentation lines up as before.
enum Out {
    /// A value as read.
    Kept(Box<RawValue>),
    /// An object Vessel changed, its keys in their order.
    Map(Vec<(String, Out)>),
    /// A list Vessel changed.
    List(Vec<Out>),
    /// Vessel's own entry.
    Vessel(Entry),
}

impl Serialize for Out {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Kept(raw) => raw.serialize(serializer),
            Self::Map(entries) => {
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (key, value) in entries {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
            Self::List(items) => serializer.collect_seq(items),
            Self::Vessel(entry) => entry.serialize(serializer),
        }
    }
}
