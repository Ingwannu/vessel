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
//! - Bounds: four listings node-wide and sixteen waiting, each at most a
//!   ranked listing's time (`places`, ADR 0109; ADR 0053 had two and
//!   nothing waiting); a cursor of at most
//!   `GetAgentHistory::MAX_CURSOR_BYTES`; an absolute folder; at most
//!   `AgentHistoryPage::MAX_ENTRIES` entries; the handshake timeout bounds
//!   the agent.
//! - Ranked (ADR 0109, `limit`): the conversations already on the node in
//!   this account are given to the listing, which leaves them out and
//!   counts them; what it answers is sorted newest first (Paseo sorts by
//!   last activity after listing; no time sorts last), each with the
//!   prompts it read. Its previews are kept per harness and account
//!   (`places::Listings::previews`).
//! - Refused to agents (`workspace::policy`) as a person's view, not as a
//!   boundary: an agent can read its harness's files with its own tools.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use vessel_agent::{HistoryFrom, NotACursor};
use vessel_proto::history::{AgentHistoryPage, GetAgentHistory, HistoryEntry};
use vessel_proto::{ErrorCode, HarnessRef, HarnessSessionId, SessionId, SessionTitle, WireError};

mod known;
mod places;

pub(super) use places::Listings;

use super::{Connection, NodeService, wire};

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
            limit,
            query,
        } = request;
        let ranked = ranked_ask(limit, query.as_deref(), cursor.is_some())?;
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
        let from = from_cursor(cursor)?;
        let Some(_place) = self.listings.place().await else {
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
        if let Some((limit, query)) = ranked {
            let ask = self
                .ranking(&harness, (&account, &implicit), limit, query.as_deref())
                .await;
            let ask = match ask {
                Ok(ask) => ask,
                Err(error) => {
                    self.models.retire(listing);
                    return Err(error);
                }
            };
            let listed = self
                .sessions
                .ranked_history(
                    harness.clone(),
                    (Some(resolved), proxy),
                    dir,
                    folder.as_deref(),
                    &ask,
                )
                .await;
            self.models.retire(listing);
            return Ok(ranked_page(listed.map_err(|error| wire(&error))?));
        }
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
        Ok(cursor_page(page, &known))
    }

    /// A ranked listing's ask: `limit` and `query`, the node's previews,
    /// and the conversations already here in this account, left out.
    async fn ranking(
        &self,
        harness: &HarnessRef,
        (account, implicit): (&vessel_proto::ProfileName, &vessel_proto::ProfileName),
        limit: usize,
        query: Option<&str>,
    ) -> Result<vessel_agent::Ranked, WireError> {
        let known = self.known_conversations(harness, account, implicit).await?;
        Ok(vessel_agent::Ranked::new(
            limit,
            query,
            self.listings.previews.clone(),
            format!("{harness}/{account}"),
        )
        .without(known.into_keys().collect()))
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

/// Where a cursor listing starts: the newest without a cursor.
fn from_cursor(cursor: Option<String>) -> Result<HistoryFrom, WireError> {
    match cursor {
        None => Ok(HistoryFrom::default()),
        Some(cursor) if cursor.len() > GetAgentHistory::MAX_CURSOR_BYTES => Err(invalid(format!(
            "a cursor is at most {} bytes",
            GetAgentHistory::MAX_CURSOR_BYTES
        ))),
        Some(cursor) => cursor
            .parse()
            .map_err(|error: NotACursor| invalid(error.to_string())),
    }
}

/// A cursor listing as its reply, each entry marked with the session
/// already on it here.
fn cursor_page(
    page: vessel_agent::HistoryPage,
    known: &HashMap<HarnessSessionId, SessionId>,
) -> AgentHistoryPage {
    AgentHistoryPage {
        conversations: page
            .entries
            .into_iter()
            .take(AgentHistoryPage::MAX_ENTRIES)
            .map(|entry| HistoryEntry {
                in_vessel: known.get(&entry.session).copied(),
                title: entry.title.as_deref().and_then(SessionTitle::from_prompt),
                updated_at: entry.updated_at.as_deref().and_then(at),
                harness_session: entry.session,
                cwd: entry.cwd,
                first_prompt: None,
                last_prompt: None,
            })
            .collect(),
        next: page.next.map(|next| next.to_string()),
        already_imported: None,
    }
}

/// A ranked listing's limit and query, checked; `None` for a cursor
/// listing.
fn ranked_ask(
    limit: Option<u16>,
    query: Option<&str>,
    cursor: bool,
) -> Result<Option<(usize, Option<String>)>, WireError> {
    let Some(limit) = limit else {
        return match query {
            Some(_) => Err(invalid("`query` needs `limit`".to_owned())),
            None => Ok(None),
        };
    };
    if cursor {
        return Err(invalid(
            "a ranked listing (`limit`) takes no `cursor`: ask a larger `limit` for more"
                .to_owned(),
        ));
    }
    if !(1..=GetAgentHistory::MAX_LIMIT).contains(&limit) {
        return Err(invalid(format!(
            "`limit` is 1 to {}, not {limit}",
            GetAgentHistory::MAX_LIMIT
        )));
    }
    if let Some(query) = query
        && query.len() > GetAgentHistory::MAX_QUERY_BYTES
    {
        return Err(invalid(format!(
            "a query is at most {} bytes",
            GetAgentHistory::MAX_QUERY_BYTES
        )));
    }
    Ok(Some((usize::from(limit), query.map(str::to_owned))))
}

/// A ranked listing as its reply: newest first (no time last, the
/// agent's order kept among equals), with the count left out.
fn ranked_page(page: vessel_agent::RankedPage) -> AgentHistoryPage {
    let mut conversations: Vec<HistoryEntry> = page
        .entries
        .into_iter()
        .take(usize::from(GetAgentHistory::MAX_LIMIT))
        .map(|entry| HistoryEntry {
            in_vessel: None,
            title: entry.title.as_deref().and_then(SessionTitle::from_prompt),
            updated_at: entry.updated_at.as_deref().and_then(at),
            harness_session: entry.session,
            cwd: entry.cwd,
            first_prompt: entry.first_prompt,
            last_prompt: entry.last_prompt,
        })
        .collect();
    conversations.sort_by_key(|entry| std::cmp::Reverse(entry.updated_at));
    AgentHistoryPage {
        conversations,
        next: None,
        already_imported: Some(u32::try_from(page.already_imported).unwrap_or(u32::MAX)),
    }
}

/// An agent's time, if it is one.
fn at(text: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|at| at.to_utc())
}

/// An `invalid_request` saying `why`.
fn invalid(why: String) -> WireError {
    WireError::new(ErrorCode::InvalidRequest, why)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn a_ranked_ask_is_checked_before_anything_starts() {
        assert_eq!(ranked_ask(None, None, true).unwrap(), None);
        assert_eq!(
            ranked_ask(Some(15), Some("build"), false).unwrap(),
            Some((15, Some("build".to_owned())))
        );
        for (limit, query, cursor) in [
            (None, Some("build"), false),
            (Some(15), None, true),
            (Some(0), None, false),
            (Some(201), None, false),
        ] {
            let refused = ranked_ask(limit, query, cursor).unwrap_err();
            assert_eq!(
                refused.code,
                ErrorCode::InvalidRequest,
                "{limit:?} {query:?}"
            );
        }
        let long = "x".repeat(GetAgentHistory::MAX_QUERY_BYTES.saturating_add(1));
        assert!(ranked_ask(Some(15), Some(&long), false).is_err());
    }

    #[test]
    fn a_ranked_page_is_newest_first_and_says_what_it_left_out() {
        let entry = |id: &str, at: Option<&str>| vessel_agent::HistoryEntry {
            session: HarnessSessionId::new(id),
            cwd: PathBuf::from("/work/app"),
            title: Some(format!("title {id}")),
            updated_at: at.map(str::to_owned),
            first_prompt: Some(format!("first {id}")),
            last_prompt: None,
        };
        let page = ranked_page(vessel_agent::RankedPage {
            entries: vec![
                entry("untimed", None),
                entry("older", Some("2026-11-02T09:00:00Z")),
                entry("newer", Some("2026-11-03T09:00:00Z")),
            ],
            already_imported: 2,
        });
        let ids: Vec<_> = page
            .conversations
            .iter()
            .map(|entry| entry.harness_session.as_str())
            .collect();
        assert_eq!(ids, ["newer", "older", "untimed"]);
        assert_eq!(page.already_imported, Some(2));
        assert_eq!(page.next, None);
        assert_eq!(
            page.conversations[0].first_prompt.as_deref(),
            Some("first newer")
        );
    }
}
