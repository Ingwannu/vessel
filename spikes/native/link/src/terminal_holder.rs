//! Which session holds a terminal that a notice names, when the notice's
//! folder place matches no session's own folder (app review of claims row
//! 3, note 2).
//!
//! [Decision Log]
//! - Purpose: a click on a terminal notice opens the terminal in the
//!   workspace of a session that holds it. Paseo's daemon puts the
//!   workspace in the notification itself (`session-context.tsx:658-677`,
//!   `workspaceId`). Vessel's notice carries only the terminal's place.
//! - Constraint: a node keeps a folder place canonical (ADR 0121,
//!   `canonical_folder`), while a session keeps its folder as it was given.
//!   Through a link or a `..` the two differ, and the app cannot resolve
//!   a path on another machine. Paths are already compared component by
//!   component, so a trailing separator or a `.` is not the problem.
//! - Alternatives: resolve the path in the app, which is right only for
//!   this computer's node; send the session or the workspace with each
//!   notice, which changes the wire; ask the node.
//! - Choice: ask the node. `terminal.list` with a folder place matches that
//!   folder canonically, as the node keeps it. Each candidate folder is
//!   listed in turn, and the first that lists the terminal names the
//!   holder ([`Located`]).
//! - Cost: one call per folder session on the machine, and only for a
//!   click that found no holder. A candidate that fails or times out is
//!   passed over, and a click that finds none opens nothing, as before.

use std::path::PathBuf;

use vessel_proto::{SessionId, TerminalId};

/// Find which of a machine's folder sessions holds `terminal`.
#[derive(Debug, Clone)]
pub struct Locate {
    /// The machine, by its book name.
    pub machine: String,
    /// The terminal the notice names.
    pub terminal: TerminalId,
    /// Each candidate session with its own folder, in the order to try.
    pub candidates: Vec<(SessionId, PathBuf)>,
}

/// The session whose folder the node matched to the terminal's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located {
    /// The machine.
    pub machine: String,
    /// The terminal.
    pub terminal: TerminalId,
    /// The session that holds it.
    pub session: SessionId,
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    /// Look for `locate`'s holder in the background; a find comes back as
    /// [`crate::Event::TerminalLocated`], and nothing comes when none is.
    pub(crate) fn locate_terminal(&self, locate: Locate) {
        let Ok(client) = self.client(&locate.machine).cloned() else {
            return;
        };
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            if let Some(located) = locate_on(&client, locate).await {
                // The receiver is gone only when the app is closing.
                let _closed = events.send(crate::Event::TerminalLocated(located)).await;
            }
        }));
    }
}

/// The first candidate whose folder lists the terminal.
#[cfg(not(target_arch = "wasm32"))]
async fn locate_on(client: &vessel_client::NodeClient, locate: Locate) -> Option<Located> {
    use vessel_proto::terminals::{ListTerminals, TerminalPlace};

    /// A call's bound, as the terminal tabs' calls.
    const CALL: std::time::Duration = std::time::Duration::from_secs(15);

    for (session, path) in locate.candidates {
        let asked = client.call(ListTerminals {
            place: Some(TerminalPlace::Folder { path }),
            changed_from: None,
        });
        match tokio::time::timeout(CALL, asked).await {
            Ok(Ok(list)) if list.terminals.iter().any(|info| info.id == locate.terminal) => {
                return Some(Located {
                    machine: locate.machine,
                    terminal: locate.terminal,
                    session,
                });
            }
            Ok(Ok(_elsewhere)) => {}
            Ok(Err(error)) => tracing::debug!(%error, "a folder's terminals were not listed"),
            Err(_elapsed) => tracing::debug!("a folder's terminal list timed out"),
        }
    }
    None
}

#[cfg(test)]
mod tests;
