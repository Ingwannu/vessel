//! Settings > Machines (the page's `data/machines.ts` and `nodes.ts`): a
//! machine's row, its state in a word, what can be done to it, the rule
//! for its name, and which machine a section is about.
//!
//! [Decision Log]
//! - Purpose: the words a machine's row says, and the machine meant when
//!   none is named, are the same on every surface.
//! - Constraint: the page's bridge lists a "here" node (the machine it
//!   runs on, outside the book); the app reads only the book, so every
//!   machine can be renamed and forgotten.
//! - Choice: the page's rules on plain inputs ([`BookNode`],
//!   [`NodeStatus`]) the app fills from its book and its last read; the
//!   sentences come from [`crate::Lang::machines`].

use crate::Lang;

/// How the book reaches a machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Transport {
    /// A WebSocket address, with a token.
    Direct,
    /// Over SSH.
    Ssh,
    /// A paired device's secure channel.
    Paired,
}

/// A machine in the book, as Settings needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookNode {
    /// Its name in the book.
    pub name: String,
    /// How it is reached.
    pub transport: Transport,
    /// Whether it is the book's default.
    pub default: bool,
}

/// What the app last knew of a machine's connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeStatus {
    /// Not answered yet.
    Connecting,
    /// Answered.
    Connected {
        /// Its system as the node names it (`linux`, `macos`).
        os: String,
        /// Its architecture (`x86_64`).
        arch: String,
        /// Whether it is draining (finishing its sessions to stop).
        draining: bool,
    },
    /// Lost, trying again.
    Reconnecting,
    /// Could not be reached.
    Offline,
}

/// A machine's state in one word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MachineState {
    /// Not answered yet.
    Connecting,
    /// Connected, finishing its sessions to stop.
    Draining,
    /// Connected.
    Connected,
    /// Lost, trying again.
    Reconnecting,
    /// Could not be reached.
    Offline,
}

/// The colour a state is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StateTone {
    /// Fine.
    Ok,
    /// Worth a look.
    Warn,
    /// Nothing to do.
    Neutral,
}

impl MachineState {
    /// The state of a machine whose status is `status` (`None`: not read
    /// yet).
    #[must_use]
    pub const fn of(status: Option<&NodeStatus>) -> Self {
        match status {
            None | Some(NodeStatus::Connecting) => Self::Connecting,
            Some(NodeStatus::Connected { draining: true, .. }) => Self::Draining,
            Some(NodeStatus::Connected {
                draining: false, ..
            }) => Self::Connected,
            Some(NodeStatus::Reconnecting) => Self::Reconnecting,
            Some(NodeStatus::Offline) => Self::Offline,
        }
    }

    /// Its colour.
    #[must_use]
    pub const fn tone(self) -> StateTone {
        match self {
            Self::Connected => StateTone::Ok,
            Self::Connecting | Self::Draining | Self::Reconnecting => StateTone::Warn,
            Self::Offline => StateTone::Neutral,
        }
    }
}

/// A system's name for people.
#[must_use]
pub fn system_name(os: &str) -> String {
    match os {
        "linux" => "Linux",
        "macos" => "macOS",
        "windows" => "Windows",
        "freebsd" => "FreeBSD",
        other => other,
    }
    .to_owned()
}

/// What a connected machine runs on (`Linux x86_64`).
#[must_use]
pub fn platform(status: Option<&NodeStatus>) -> Option<String> {
    match status {
        Some(NodeStatus::Connected { os, arch, .. }) => Some(format!("{} {arch}", system_name(os))),
        Some(NodeStatus::Connecting | NodeStatus::Reconnecting | NodeStatus::Offline) | None => {
            None
        }
    }
}

/// A machine's row: what it runs on, how it is reached, its state, and its
/// live sessions while connected.
#[must_use]
pub fn machine_line(
    lang: Lang,
    node: &BookNode,
    status: Option<&NodeStatus>,
    live: usize,
) -> Vec<String> {
    let words = lang.machines();
    let connected = matches!(status, Some(NodeStatus::Connected { .. }));
    platform(status)
        .into_iter()
        .chain([
            words.reach(node.transport).to_owned(),
            words.state(MachineState::of(status)).to_owned(),
        ])
        .chain(connected.then(|| words.live_count(live)))
        .collect()
}

/// What Settings offers to do to a machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MachineActions {
    /// Rename it in the book.
    pub rename: bool,
    /// Make it the book's default.
    pub make_default: bool,
    /// Forget it (behind a confirm).
    pub forget: bool,
}

/// What can be done to `node`.
#[must_use]
pub const fn machine_actions(node: &BookNode) -> MachineActions {
    MachineActions {
        rename: true,
        make_default: !node.default,
        forget: true,
    }
}

/// The longest name, in characters.
pub const MAX_NAME: usize = 64;

/// Why a name cannot be a machine's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameProblem {
    /// Nothing typed.
    Empty,
    /// It does not start with a letter or a digit.
    Start,
    /// A character other than letters, digits, `.`, `_` and `-`.
    Characters,
    /// More than [`MAX_NAME`] characters.
    TooLong,
    /// Another machine has it.
    Taken(String),
}

/// What is wrong with `text` as a machine's name in `book`, if anything;
/// `keep` is the machine being renamed (its own name is not taken).
#[must_use]
pub fn name_problem(text: &str, book: &[BookNode], keep: Option<&str>) -> Option<NameProblem> {
    let allowed = |byte: u8| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-');
    if text.is_empty() {
        return Some(NameProblem::Empty);
    }
    if !text
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_alphanumeric())
    {
        return Some(NameProblem::Start);
    }
    if !text.bytes().all(allowed) {
        return Some(NameProblem::Characters);
    }
    if text.len() > MAX_NAME {
        return Some(NameProblem::TooLong);
    }
    let taken = book
        .iter()
        .any(|node| node.name == text && Some(node.name.as_str()) != keep);
    taken.then(|| NameProblem::Taken(text.to_owned()))
}

/// The machine meant: the first of `wanted` that is given and accepted,
/// else the book's default, else the first in the book that is.
#[must_use]
pub fn machine_meant<'a>(
    book: &'a [BookNode],
    wanted: &[Option<&'a str>],
    accept: impl Fn(&str) -> bool,
) -> Option<&'a str> {
    let default = book
        .iter()
        .find(|node| node.default)
        .map(|node| node.name.as_str());
    wanted
        .iter()
        .copied()
        .chain([default])
        .chain(book.iter().map(|node| Some(node.name.as_str())))
        .flatten()
        .find(|name| accept(name))
}

/// The machine a section is about: the one the address asks for, else the
/// last chosen, else the default, among those in the book.
#[must_use]
pub fn machine_for<'a>(
    book: &'a [BookNode],
    asked: Option<&'a str>,
    selected: Option<&'a str>,
) -> Option<&'a BookNode> {
    let name = machine_meant(book, &[asked, selected], |candidate| {
        book.iter().any(|node| node.name == candidate)
    })?;
    book.iter().find(|node| node.name == name)
}

/// A machine offered by a picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineChoice {
    /// Its name in the book.
    pub value: String,
    /// What the picker shows.
    pub label: String,
    /// Its state, when it is not connected.
    pub detail: Option<String>,
    /// Whether it cannot be chosen (not connected, for a new session).
    pub disabled: bool,
}

/// The book's machines for a picker, each not connected saying why;
/// `connected_only` turns those off.
#[must_use]
pub fn machine_choices(
    lang: Lang,
    book: &[BookNode],
    status_of: impl Fn(&str) -> Option<NodeStatus>,
    connected_only: bool,
) -> Vec<MachineChoice> {
    book.iter()
        .map(|node| {
            let status = status_of(&node.name);
            let connected = matches!(status, Some(NodeStatus::Connected { .. }));
            MachineChoice {
                value: node.name.clone(),
                label: node.name.clone(),
                detail: (!connected).then(|| {
                    lang.machines()
                        .state(MachineState::of(status.as_ref()))
                        .to_owned()
                }),
                disabled: !connected && connected_only,
            }
        })
        .collect()
}
