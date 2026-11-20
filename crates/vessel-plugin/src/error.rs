//! A create hook failed. The handler's own text is not a field: a wire
//! message or a log line must not carry a path or a secret the handler
//! wrote.

use std::fmt;

use crate::hook::Hook;
use crate::id::PluginId;

/// A spec field the create-before hook may read and must not replace.
///
/// Title and harness are the two the handler may replace. Every other
/// field of the spec is locked, including `side_question_of`, which the
/// record's list of examples does not name and the same rule covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockedField {
    /// `SessionSpec.cwd`
    Cwd,
    /// `SessionSpec.place`
    Place,
    /// `SessionSpec.profile`
    Profile,
    /// `SessionSpec.mode`
    Mode,
    /// `SessionSpec.side_question_of`
    Side,
}

impl LockedField {
    /// The field's name, never its value.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cwd => "cwd",
            Self::Place => "place",
            Self::Profile => "profile",
            Self::Mode => "mode",
            Self::Side => "side_question_of",
        }
    }
}

impl fmt::Display for LockedField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// Why a hook call did not finish as asked.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PluginError {
    /// The id was empty.
    #[error("a plugin id is empty")]
    EmptyId,
    /// The id is outside `^[a-z][a-z0-9-]*$`. The text is not repeated.
    #[error("a plugin id is not acceptable")]
    BadId,
    /// Registration order no longer fits. Tests never reach this.
    #[error("the plugin registry cannot take another handler")]
    Full,
    /// An index the registry itself built was missing.
    #[error("the plugin registry lost an entry")]
    Broken,
    /// The handler refused the create. Its text is not kept.
    #[error("plugin {plugin} hook {hook} refused")]
    Refused {
        /// Which plugin.
        plugin: PluginId,
        /// Which hook.
        hook: Hook,
    },
    /// The handler tried to replace a locked field. The value is not kept.
    #[error("plugin {plugin} hook {hook} refused to change {field}")]
    Changed {
        /// Which plugin.
        plugin: PluginId,
        /// Which hook.
        hook: Hook,
        /// Which field, by name.
        field: LockedField,
    },
    /// The node's cancellation token fired while this handler ran.
    #[error("plugin {plugin} hook {hook} was cancelled")]
    Cancelled {
        /// Which plugin.
        plugin: PluginId,
        /// Which hook.
        hook: Hook,
    },
}

/// The handler refused. It carries no text on purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandlerStop;
