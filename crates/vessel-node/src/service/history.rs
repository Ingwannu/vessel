//! Protocol 17's `agent.history` (track A2, ADR 0053): a page of an
//! agent's own conversations, so a person imports one (`session.create`
//! with `resume`). Routing and bounds here; the listing itself is
//! `vessel_agent::history`, started by the session runtime as a probe is.
//!
//! [Decision Log]
//! - Purpose: the list a client offers to import from, marked with what
//!   this node already has, so the same conversation is not imported twice
//!   without the person seeing it.
//! - Choice: the agent is started under the profile in the probe folder
//!   (as `harness.options`, with a routed profile's model token), never
//!   cached (a conversation just had must show). `in_vessel` comes from
//!   the node's sessions, live and kept: the newest of this harness whose
//!   `harness_session` is the entry's, one scan of them a page (no index:
//!   a node keeps at most a few hundred).
//! - Bounds: [`AT_ONCE`] listings node-wide, more answered `unavailable`
//!   (as searches are; a queue per profile would wait without bound); a
//!   cursor of at most `GetAgentHistory::MAX_CURSOR_BYTES`; an absolute
//!   folder; at most `AgentHistoryPage::MAX_ENTRIES` entries; the
//!   handshake timeout bounds the agent.
//! - Refused to agents (`workspace::policy`) as a person's view, not as a
//!   boundary: an agent can read its harness's files with its own tools.

use std::collections::HashMap;

use vessel_agent::{HistoryFrom, NotACursor};
use vessel_proto::history::{AgentHistoryPage, GetAgentHistory, HistoryEntry};
use vessel_proto::{ErrorCode, HarnessRef, HarnessSessionId, SessionId, SessionTitle, WireError};

mod known;

use super::{Connection, NodeService, wire};

/// Listings running at once, node-wide; more are `unavailable`.
pub(super) const AT_ONCE: usize = 2;

impl NodeService {
    /// `agent.history`: a page of `harness`'s conversations under
    /// `profile`, each marked with the session already on it here.
    pub(crate) async fn agent_history(
        &self,
        _: &Connection,
        request: GetAgentHistory,
    ) -> Result<AgentHistoryPage, WireError> {
        let GetAgentHistory {
            harness,
            profile,
            cwd,
            cursor,
        } = request;
        let folder = match cwd {
            Some(cwd) if !cwd.is_absolute() => {
                return Err(invalid(format!(
                    "`cwd` must be an absolute folder, not {}",
                    cwd.display()
                )));
            }
            // The agents match the folder as written (Claude's exactly), and
            // a session records it resolved: a link resolved the same way.
            // A folder gone since is asked as given.
            Some(cwd) => Some(tokio::fs::canonicalize(&cwd).await.unwrap_or(cwd)),
            None => None,
        };
        let from = match cursor {
            None => HistoryFrom::default(),
            Some(cursor) if cursor.len() > GetAgentHistory::MAX_CURSOR_BYTES => {
                return Err(invalid(format!(
                    "a cursor is at most {} bytes",
                    GetAgentHistory::MAX_CURSOR_BYTES
                )));
            }
            Some(cursor) => cursor
                .parse()
                .map_err(|error: NotACursor| invalid(error.to_string()))?,
        };
        let Ok(_place) = self.listings.try_acquire() else {
            return Err(WireError::new(
                ErrorCode::Unavailable,
                "the node is listing as many agents' histories as it lists at once; ask again",
            ));
        };
        self.refuse_installing(&harness)
            .map_err(|error| wire(&error))?;
        let resolved = self
            .resolve(&harness, profile.as_ref())
            .map_err(|error| wire(&error))?;
        let account = resolved.name().clone();
        let implicit = self
            .resolve(&harness, None)
            .map_err(|error| wire(&error))?
            .name()
            .clone();
        // A routed profile's agent gets a token of its own while it lists,
        // retired after, as for a probe: no prompt is sent.
        let listing = SessionId::random();
        let proxy = self
            .models
            .launch(Some(&resolved), listing)
            .map_err(|error| wire(&error))?
            .map(|(_model, proxy)| proxy);
        let dir = self.probes.dir().clone();
        let listed = self
            .sessions
            .history(
                harness.clone(),
                (Some(resolved), proxy),
                dir,
                folder.as_deref(),
                from,
            )
            .await;
        self.models.retire(listing);
        let page = listed.map_err(|error| wire(&error))?;
        let known = self
            .known_conversations(&harness, &account, &implicit)
            .await?;
        Ok(AgentHistoryPage {
            conversations: page
                .entries
                .into_iter()
                .take(AgentHistoryPage::MAX_ENTRIES)
                .map(|entry| HistoryEntry {
                    in_vessel: known.get(&entry.session).copied(),
                    title: entry.title.as_deref().and_then(SessionTitle::from_prompt),
                    updated_at: entry.updated_at.as_deref().and_then(|at| {
                        chrono::DateTime::parse_from_rfc3339(at)
                            .ok()
                            .map(|at| at.to_utc())
                    }),
                    harness_session: entry.session,
                    cwd: entry.cwd,
                })
                .collect(),
            next: page.next.map(|next| next.to_string()),
        })
    }

    /// A transcript copy in another account must not appear already imported.
    async fn known_conversations(
        &self,
        harness: &HarnessRef,
        account: &vessel_proto::ProfileName,
        implicit: &vessel_proto::ProfileName,
    ) -> Result<HashMap<HarnessSessionId, SessionId>, WireError> {
        let sessions = self.sessions.list().await.map_err(|error| wire(&error))?;
        Ok(known::in_account(sessions, harness, account, implicit))
    }
}

/// An `invalid_request` saying `why`.
fn invalid(why: String) -> WireError {
    WireError::new(ErrorCode::InvalidRequest, why)
}
