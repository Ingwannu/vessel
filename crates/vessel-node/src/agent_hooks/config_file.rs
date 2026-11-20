//! Claude Code's `settings.json` and Codex's `hooks.json`: a `hooks` object
//! with one list of matcher entries per event (Paseo's
//! `claude/claude-settings.js` and `codex/codex-settings.js`).
//!
//! [Decision Log]
//! - Purpose: add Vessel's entry to each event, or take it out, and change
//!   nothing else in a file that is a person's own.
//! - Paseo parses the whole file, replaces its marked entries, and writes
//!   it back with `JSON.stringify(config, null, 2)`. A file that is not an
//!   object becomes `{}`.
//! - Choice:
//!   - The file is read as an object that keeps its keys in order and each
//!     value's text as written ([`Object`], on `serde_json`'s `RawValue`).
//!     Only `hooks` and the lists of Vessel's own events are rebuilt, and in
//!     those only entries that carry Vessel's marker change. Everything
//!     else is written back as it was. `serde_json`'s map would sort the
//!     keys of the whole file.
//!   - A file that is not JSON, not an object, or whose `hooks` or event
//!     list has another shape is left untouched ([`Outcome::Invalid`]).
//!     Paseo would replace it.
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
    let mut config = match current {
        Some(text) => serde_json::from_str::<Object>(text).ok()?,
        None => Object::default(),
    };
    let had_hooks = config.get("hooks").is_some();
    let mut hooks = match config.get("hooks") {
        Some(raw) => serde_json::from_str::<Object>(raw.get()).ok()?,
        None => Object::default(),
    };
    for event in agent.event_names() {
        let listed = match hooks.get(event) {
            Some(raw) => serde_json::from_str::<Vec<Box<RawValue>>>(raw.get()).ok()?,
            None => Vec::new(),
        };
        let mut entries = Vec::with_capacity(listed.len().saturating_add(1));
        for entry in listed {
            match unmarked(agent, entry)? {
                Kept::Entry(kept) => entries.push(kept),
                Kept::Gone => {}
            }
        }
        if on {
            entries.push(serde_json::value::to_raw_value(&entry(agent, event)).ok()?);
        }
        if entries.is_empty() {
            hooks.remove(event);
        } else {
            hooks.set(event, serde_json::value::to_raw_value(&entries).ok()?);
        }
    }
    if had_hooks || !hooks.0.is_empty() {
        config.set("hooks", serde_json::value::to_raw_value(&hooks).ok()?);
    }
    let mut text = serde_json::to_string_pretty(&config).ok()?;
    text.push('\n');
    Some(text)
}

/// What one listed entry becomes.
enum Kept {
    /// Written back: as it was, or without Vessel's commands.
    Entry(Box<RawValue>),
    /// Vessel's commands were all it had.
    Gone,
}

/// `entry` without Vessel's commands: as written when it has none, rebuilt
/// without them when it has others too, and [`Kept::Gone`] when they were
/// all it had. `None` only when a rebuilt entry cannot be written.
fn unmarked(agent: Agent, entry: Box<RawValue>) -> Option<Kept> {
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(entry.get()) else {
        return Some(Kept::Entry(entry));
    };
    let Some(commands) = value
        .get_mut("hooks")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return Some(Kept::Entry(entry));
    };
    let before = commands.len();
    commands.retain(|command| !ours(agent, command));
    if commands.len() == before {
        return Some(Kept::Entry(entry));
    }
    if commands.is_empty() {
        return Some(Kept::Gone);
    }
    serde_json::value::to_raw_value(&value)
        .ok()
        .map(Kept::Entry)
}

/// Whether a hook command is one Vessel installed for `agent`.
fn ours(agent: Agent, command: &serde_json::Value) -> bool {
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
pub(super) struct Object(Vec<(String, Box<RawValue>)>);

impl Object {
    fn get(&self, key: &str) -> Option<&RawValue> {
        self.0
            .iter()
            .find(|(name, _value)| name == key)
            .map(|(_name, value)| value.as_ref())
    }

    /// Replace `key`'s value where it is, or add it at the end.
    fn set(&mut self, key: &str, value: Box<RawValue>) {
        match self.0.iter_mut().find(|(name, _value)| name == key) {
            Some((_name, kept)) => *kept = value,
            None => self.0.push((key.to_owned(), value)),
        }
    }

    fn remove(&mut self, key: &str) {
        self.0.retain(|(name, _value)| name != key);
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

impl Serialize for Object {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in &self.0 {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}
