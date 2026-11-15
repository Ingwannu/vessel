//! The panel's file finding for other areas: track P's `@` in the composer
//! (Paseo's `utils/file-mention-autocomplete.ts`) asks `code_find_files`
//! with its own key, and the answers that are not the panel's go to
//! `found_elsewhere`, P's slot. The open session's root is read once per
//! session (`session.get`), whether or not the panel is open.
//!
//! [Decision Log]
//! - Purpose: `@` a file of the project from the composer, with no second
//!   way to reach the node's index or to learn where a session runs.
//! - Constraint: a session's card has no root; the panel learns it with
//!   `session.get` only when it opens. The node stops an older find only
//!   when a newer one comes with the same key, so each field has its own.
//! - Alternatives: P reading `session.get` itself (two caches of the same
//!   fact); the root on the card (the lead's and the protocol's).
//! - Choice: one cache of each session's root here, filled by every
//!   `session.get` answer this area gets; a find asked before the root is
//!   known waits for it (the newest query only) and is sent when it comes.
//! - Cost: the first `@` in a session waits for one more round trip, and
//!   says so (`FindRefused::Reading`).

use std::collections::{HashMap, HashSet};

use vessel_app_link::workspace::{Ask, FileAsk};
use vessel_app_view::workspace::problem::Problem;
use vessel_app_view::workspace::root::session_root;
use vessel_proto::workspace::{FindFiles, FindQuery, FoundFiles, SearchKey, WorkspaceRoot};
use vessel_proto::{SessionId, SessionInfo};

use super::Place;
use crate::shell::Shell;

/// Why a find was not sent.
#[expect(dead_code, reason = "track P's seam: its first caller is P's `@`")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FindRefused {
    /// No session is open: `@` has no project to look in.
    NoSession,
    /// The session's root is being read; the find goes out when it comes.
    Reading,
    /// The session runs in no project.
    Outside,
    /// The query is not one the node takes (a line of at most 256).
    Query,
    /// Where the session runs could not be read; why (the machine's
    /// words). The next call reads it again.
    Failed(String),
}

/// A find waiting for its session's root.
struct Waiting {
    machine: String,
    session: SessionId,
    key: SearchKey,
    query: FindQuery,
    limit: u32,
}

/// Each session's root as read, and the find waiting on one.
#[derive(Default)]
pub(super) struct Finder {
    /// `Ok(None)`: in no project; `Err`: the read failed, and why.
    roots: HashMap<SessionId, Result<Option<WorkspaceRoot>, String>>,
    /// Sessions whose `session.get` is out.
    asked: HashSet<SessionId>,
    waiting: Option<Waiting>,
}

impl Shell {
    /// Track P's call: find files of the open session's project by name,
    /// the best `limit` first, under `key` (P's own, from `SearchKeys`).
    /// The answer comes to `found_elsewhere`.
    #[expect(dead_code, reason = "track P's seam: its first caller is P's `@`")]
    pub(crate) fn code_find_files(
        &mut self,
        key: SearchKey,
        query: &str,
        limit: u32,
    ) -> Result<(), FindRefused> {
        let Some((machine, session)) = self.code_session() else {
            return Err(FindRefused::NoSession);
        };
        let Ok(query) = query.trim().parse::<FindQuery>() else {
            return Err(FindRefused::Query);
        };
        let panel = &mut self.areas.workspace.code;
        // The panel knows it already when it shows this session.
        let shown = panel.target.as_ref().and_then(|target| {
            if target.session != session {
                return None;
            }
            match &target.place {
                Place::Root(root) => Some(Ok(Some(*root))),
                Place::Outside => Some(Ok(None)),
                Place::Reading | Place::Failed(_) => None,
            }
        });
        let finder = &mut panel.finder;
        match shown.or_else(|| finder.roots.get(&session).cloned()) {
            Some(Ok(Some(root))) => {
                self.ask(&machine, find_ask(root, query, key, limit));
                Ok(())
            }
            Some(Ok(None)) => Err(FindRefused::Outside),
            Some(Err(why)) => {
                let _was = finder.roots.remove(&session);
                Err(FindRefused::Failed(why))
            }
            None => {
                finder.waiting = Some(Waiting {
                    machine: machine.clone(),
                    session,
                    key,
                    query,
                    limit,
                });
                if finder.asked.insert(session) {
                    self.ask(&machine, Ask::Session(session));
                }
                Err(FindRefused::Reading)
            }
        }
    }

    /// A session's root arrived (for the panel or for a find): kept, and
    /// the find waiting on it sent.
    pub(super) fn finder_session_read(
        &mut self,
        session: SessionId,
        result: &Result<Box<SessionInfo>, Problem>,
    ) {
        let words = self.words();
        let finder = &mut self.areas.workspace.code.finder;
        let _was = finder.asked.remove(&session);
        let root = match result {
            Ok(info) => Ok(session_root(info)),
            Err(problem) => Err(words.problem(problem)),
        };
        let _was = finder.roots.insert(session, root.clone());
        let Some(waiting) = finder.waiting.take_if(|waiting| waiting.session == session) else {
            return;
        };
        // In no project or not read: the next call says why.
        if let Ok(Some(root)) = root {
            self.ask(
                &waiting.machine,
                find_ask(root, waiting.query, waiting.key, waiting.limit),
            );
        }
    }
}

impl Finder {
    /// The machine was renamed or forgotten in the book: a find waiting
    /// on it is dropped (the roots are keyed by session, not by name; the
    /// next call asks again under the new name).
    pub(super) fn machine_gone(&mut self, name: &str) {
        let _was = self.waiting.take_if(|waiting| waiting.machine == name);
    }
}

fn find_ask(root: WorkspaceRoot, query: FindQuery, key: SearchKey, limit: u32) -> Ask {
    Ask::File(FileAsk::Find(FindFiles {
        root,
        query,
        key: Some(key),
        limit: Some(limit),
    }))
}

/// Track P's slot: a find answer under a key that is not the panel's
/// (`code_find_files`). P replaces this body with one call into the
/// composer, `shell.composer_files_found(ask, result)`.
pub(super) fn found_elsewhere(
    _shell: &mut Shell,
    _ask: &FindFiles,
    _result: Result<FoundFiles, Problem>,
) {
}
