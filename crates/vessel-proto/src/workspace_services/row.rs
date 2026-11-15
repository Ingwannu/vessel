//! A current definition and its terminal observation can evolve independently.
//! Neither a row nor its borrowed view is an execution/admission capability.

use serde::{Deserialize, Serialize};

use crate::{
    scripts::{ScriptEntry, ScriptKind},
    terminals::TerminalInfo,
};

use super::ServiceObservation;

/// Why a definition remains in a workspace's current service view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceScriptOrigin {
    /// The current decoded project file names it.
    Configured,
    /// A captured approved definition survives while its terminal is supervised.
    RemovedRunning,
    /// A newer classification must not enable an old client's mutations.
    #[serde(other)]
    Unknown,
}

/// Private fields prevent plain scripts from acquiring service observations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(try_from = "RawWorkspaceScriptRow")]
pub struct WorkspaceScriptRow {
    definition: ScriptEntry,
    origin: WorkspaceScriptOrigin,
    #[serde(skip_serializing_if = "Option::is_none")]
    terminal: Option<TerminalInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    service: Option<ServiceObservation>,
}

/// Every display field is named here; adding an owner field breaks projection.
#[derive(Debug, Clone, Copy)]
pub struct WorkspaceScriptRowView<'a> {
    /// Use ScriptEntry::view for exhaustive approved-field rendering.
    pub definition: &'a ScriptEntry,
    /// Distinguish captured removed definitions from the current file.
    pub origin: WorkspaceScriptOrigin,
    /// Supervised state is not proof of a live kernel process.
    pub terminal: Option<&'a TerminalInfo>,
    /// Missing data never defaults to a healthy/listening service.
    pub service: Option<&'a ServiceObservation>,
}

/// Refuse contradictory metadata before a consumer can render it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidWorkspaceScriptRow {
    /// No implicit discard of an unsupported service-only field.
    #[error("a plain script cannot carry service observations")]
    ServiceOnScript,
    /// A terminal is associated by its recorded script name, never cwd.
    #[error("the terminal does not name this workspace script")]
    Terminal,
    /// Stopped removed definitions disappear, as in the reference projection.
    #[error("a removed running definition needs a supervised terminal")]
    RemovedNotRunning,
}

impl WorkspaceScriptRow {
    /// Keep definition role and status metadata consistent before wire/display.
    ///
    /// # Errors
    /// A plain script with service data, unrelated terminal, or stopped orphan.
    pub fn new(
        definition: ScriptEntry,
        origin: WorkspaceScriptOrigin,
        terminal: Option<TerminalInfo>,
        service: Option<ServiceObservation>,
    ) -> Result<Self, InvalidWorkspaceScriptRow> {
        if definition.kind() == ScriptKind::Script && service.is_some() {
            return Err(InvalidWorkspaceScriptRow::ServiceOnScript);
        }
        if terminal
            .as_ref()
            .is_some_and(|info| info.kind.script() != Some(&definition.name))
        {
            return Err(InvalidWorkspaceScriptRow::Terminal);
        }
        if origin == WorkspaceScriptOrigin::RemovedRunning
            && !terminal.as_ref().is_some_and(TerminalInfo::is_live)
        {
            return Err(InvalidWorkspaceScriptRow::RemovedNotRunning);
        }
        Ok(Self {
            definition,
            origin,
            terminal,
            service,
        })
    }

    /// Preserve a complete borrowed projection without exposing mutable halves.
    #[must_use]
    pub const fn view(&self) -> WorkspaceScriptRowView<'_> {
        let Self {
            definition,
            origin,
            terminal,
            service,
        } = self;
        WorkspaceScriptRowView {
            definition,
            origin: *origin,
            terminal: terminal.as_ref(),
            service: service.as_ref(),
        }
    }
}

#[derive(Deserialize)]
struct RawWorkspaceScriptRow {
    definition: ScriptEntry,
    origin: WorkspaceScriptOrigin,
    #[serde(default)]
    terminal: Option<TerminalInfo>,
    #[serde(default)]
    service: Option<ServiceObservation>,
}

impl TryFrom<RawWorkspaceScriptRow> for WorkspaceScriptRow {
    type Error = InvalidWorkspaceScriptRow;

    fn try_from(raw: RawWorkspaceScriptRow) -> Result<Self, Self::Error> {
        Self::new(raw.definition, raw.origin, raw.terminal, raw.service)
    }
}

#[cfg(test)]
mod tests;
