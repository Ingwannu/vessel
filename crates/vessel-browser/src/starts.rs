//! The setting at session start, remembered for the life of this process.
//! A restart has no memory of it: a session with no record follows the
//! setting then in force.

use std::collections::HashMap;
use std::sync::Mutex;

use vessel_proto::browser::BrowserFailure;
use vessel_proto::settings::BrowserTools;
use vessel_proto::SessionId;

use crate::policy::Start;

/// One record per session this process started.
#[derive(Debug, Default)]
pub struct Starts {
    inner: Mutex<HashMap<SessionId, Start>>,
}

impl Starts {
    /// Remember the setting at `session`'s start.
    ///
    /// # Errors
    /// [`BrowserFailure::Internal`] when the lock was poisoned.
    pub fn record(&self, session: SessionId, setting: BrowserTools) -> Result<(), BrowserFailure> {
        let mut guard = self.inner.lock().map_err(|_| BrowserFailure::Internal)?;
        let _replaced = guard.insert(session, Start::at(setting));
        Ok(())
    }

    /// The start recorded for `session`, or [`Start::ToolsOn`] when this
    /// process did not start it, so the setting now in force applies.
    ///
    /// # Errors
    /// [`BrowserFailure::Internal`] when the lock was poisoned.
    pub fn start_of(&self, session: SessionId) -> Result<Start, BrowserFailure> {
        let guard = self.inner.lock().map_err(|_| BrowserFailure::Internal)?;
        Ok(guard.get(&session).copied().unwrap_or(Start::ToolsOn))
    }
}
