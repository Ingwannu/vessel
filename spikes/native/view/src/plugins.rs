//! Settings › Plugins (ADR 0115): a machine's plugins, as Paseo's host
//! Plugins page (`screens/settings/plugins-page.tsx`,
//! `plugins-page-state.ts`, `plugin-install-form-model.ts`) and as the
//! page's section (`web/src/data/plugins.ts`, `use-plugins.ts`).
//!
//! [Decision Log]
//! - Purpose: Paseo lists a host's plugins, switches plugins on for the
//!   host, installs one from a source, enables, disables, reloads and
//!   removes one and shows its logs. The node has the calls (`plugin.*`
//!   and `node.settings`' `plugins_enabled`, ADR 0115); the page draws
//!   them; the app did not.
//! - Existing assets: the page's model (its state order, install field,
//!   status looks and words), ported here one for one; `devices::Ticket`
//!   and `MachineKey`, which every Settings call carries so a late answer
//!   or another machine's changes nothing.
//! - Alternatives: the decisions in the app's drawing code (untestable
//!   without a window), or one module per call (the order of a change, the
//!   list read after it and the watch that follows would be spread over
//!   several).
//! - Choice: the decisions (which state the page is in, whether the field
//!   can be sent, a status's tone, which answer belongs to which request,
//!   what to ask next) are plain values and functions here, tested on
//!   their own ([`Page`]); the app sends the [`Call`]s this returns and
//!   draws what it holds.
//! - A node's error text is shown, as Paseo's `errorMessage(error)` and the
//!   page's problem message: unlike Devices or the node's settings, a
//!   plugin's failure is about code the person installed on their own
//!   machine, and the node's words (a manifest it could not read, a helper
//!   that stopped) are the only account of it. They are shown as they
//!   came, never inside a translated sentence (the locales README).
//! - Paseo pushes `plugin_catalog_changed`; the node has no node-wide push,
//!   so the page long-polls `plugin.watch` as the web page does (ADR 0115).
//! - Cost: the watch holds one request open per open page (the node
//!   answers within 25 s); it stops on a failure and starts again with the
//!   next connection.

use vessel_proto::plugins::{PluginGeneration, PluginId, PluginListItem, PluginLogs, PluginStatus};
use vessel_proto::settings::NodeSettings;
use vessel_proto::{ErrorCode, ProtocolVersion};

use crate::devices::Ticket;

mod logs;
mod page;
mod words;

pub use logs::{Logs, LogsView, log_head};
pub use page::{Answered, Page, RowLook};
pub use words::{change_words, problem_words, source_line, status_word};

/// The protocol that has the plugin calls (`PLUGINS_PROTOCOL`).
pub const PROTOCOL: Option<ProtocolVersion> = vessel_proto::plugins::PLUGINS_PROTOCOL;

/// Whether a machine speaking `protocol` serves the plugin calls (the
/// page's `speaksPlugins`). An unknown protocol is not asked.
#[must_use]
pub fn speaks(protocol: Option<ProtocolVersion>) -> bool {
    protocol
        .zip(PROTOCOL)
        .is_some_and(|(spoken, since)| spoken >= since)
}

/// Paseo's `PluginPageState`, in the order Paseo decides it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageState {
    /// Not connected: "Plugin host is offline".
    Offline,
    /// Connected to a node without the calls: "Update this machine…".
    Unsupported,
    /// The first list is being read.
    Loading,
    /// The list could not be read.
    Error,
    /// Nothing is installed.
    Empty,
    /// The installations.
    Ready,
}

/// What Paseo's `resolvePluginPageState` reads of the machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reached {
    /// It answers now.
    pub connected: bool,
    /// Its node has the plugin calls.
    pub supported: bool,
}

/// What it reads of the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Listed {
    /// No list yet.
    pub loading: bool,
    /// The last read failed.
    pub error: bool,
    /// How many installations the last read had.
    pub count: usize,
}

/// Paseo's `resolvePluginPageState`: offline, then too old, then loading,
/// then the error, then empty or the list.
#[must_use]
pub const fn page_state(machine: Reached, list: Listed) -> PageState {
    if !machine.connected {
        PageState::Offline
    } else if !machine.supported {
        PageState::Unsupported
    } else if list.loading {
        PageState::Loading
    } else if list.error {
        PageState::Error
    } else if list.count == 0 {
        PageState::Empty
    } else {
        PageState::Ready
    }
}

/// Paseo's install form: only a non-empty source is sent, trimmed
/// (`canSubmit`, `getSubmission`).
#[must_use]
pub fn install_source(text: &str) -> Option<&str> {
    Some(text.trim()).filter(|source| !source.is_empty())
}

/// A status badge's tone (Paseo: running success, failed error, otherwise
/// muted).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Running.
    Ok,
    /// Failed.
    Danger,
    /// Disabled.
    Neutral,
}

/// The tone of `status`'s badge.
#[must_use]
pub const fn status_tone(status: PluginStatus) -> Tone {
    match status {
        PluginStatus::Running => Tone::Ok,
        PluginStatus::Failed => Tone::Danger,
        PluginStatus::Disabled => Tone::Neutral,
    }
}

/// A row's action (Paseo's `PluginRowAction`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Stop it and start it again from disk.
    Reload,
    /// Save it as enabled (the row's switch on).
    Enable,
    /// Save it as disabled (the row's switch off).
    Disable,
    /// Forget it; its directory stays.
    Remove,
}

/// The change in flight: the controls wait for it (Paseo's pending
/// mutation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pending {
    /// Installing from the field.
    Install,
    /// Turning plugins on (`true`) or off for the machine.
    Switch(bool),
    /// A row's action on one installation.
    Row(Action, PluginId),
}

/// What a change did, in Paseo's words (`settings.plugins.feedback.*`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// "Installed {id}".
    Installed(PluginId),
    /// "Reloaded {id}", "Enabled {id}", "Disabled {id}", "Removed {id}".
    Acted(Action, PluginId),
    /// "Kept {id}": its removal was cancelled.
    Kept(PluginId),
    /// "Plugins enabled" or "Plugins disabled".
    Switched(bool),
}

/// The note under the install field after a change (Paseo's feedback).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Feedback {
    /// It was done: a success note.
    Done(Change),
    /// It failed: an error banner.
    Failed(Problem),
}

/// Why a call had no answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// No connection to the machine: nothing was sent.
    Offline,
    /// The connection dropped before the answer: it may have happened.
    Dropped,
    /// No answer in time: it may have happened.
    TimedOut,
    /// The machine's node does not have the call.
    Unsupported,
    /// The node refused it, with its code and its words when it gave some.
    Refused {
        /// The node's code.
        code: ErrorCode,
        /// The node's own words, shown as they came.
        message: Option<String>,
    },
}

/// A call the page makes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// `plugin.list`.
    List,
    /// `node.settings`, for the switch.
    ReadSwitch,
    /// `node.set_settings` with `plugins_enabled`.
    Switch(bool),
    /// `plugin.install` from a source.
    Install(String),
    /// `plugin.enable`, `.disable`, `.reload` or `.remove`.
    Act(Action, PluginId),
    /// `plugin.logs`.
    Logs(PluginId),
    /// `plugin.watch` after a generation.
    Watch(PluginGeneration),
}

/// What a call answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    /// The installations.
    List(Vec<PluginListItem>),
    /// The node's settings in force.
    Settings(NodeSettings),
    /// The installation a change made.
    Item(PluginListItem),
    /// A row's action was done.
    Done,
    /// An installation's log tail.
    Logs(PluginLogs),
    /// The node's plugin generation.
    Generation(PluginGeneration),
}

/// A call to send: what to ask, with the ticket its answer will carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    /// The answer's identity.
    pub ticket: Ticket,
    /// What to ask.
    pub request: Request,
}
