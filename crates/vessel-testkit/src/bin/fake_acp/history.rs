//! `history`: what `session/list` answers (protocol 17's `agent.history`).
//!
//! Three earlier conversations, newest first, two a page:
//!
//! - no cursor: `past-3` (titled `third`) and `past-2` (`second`), and an
//!   entry whose folder is relative (`relative/dir`, which a client must
//!   leave out), then cursor `page-2`;
//! - `page-2`: `past-1` (no title, no time), and no cursor.
//!
//! Each is in the request's `cwd`, or without one in the agent's own
//! working directory (where it was started). Any other cursor is
//! `invalid_params`, as an agent answers a cursor it never gave.
//! `session/load` of any of them replays as for any other id.

use std::path::PathBuf;

use agent_client_protocol::Error;
use agent_client_protocol::v1::schema::{ListSessionsResponse, SessionInfo};

/// The cursor page one hands out.
const SECOND_PAGE: &str = "page-2";

/// The page `cursor` names, its conversations in `cwd` (the agent's own
/// directory without one).
pub(crate) fn page(
    cwd: Option<PathBuf>,
    cursor: Option<&str>,
) -> Result<ListSessionsResponse, Error> {
    let cwd = match cwd {
        Some(cwd) => cwd,
        None => std::env::current_dir().map_err(|error| {
            Error::internal_error().data(format!("no working directory: {error}"))
        })?,
    };
    let entry = |id: &str, title: Option<&str>, updated: Option<&str>| {
        let mut info = SessionInfo::new(id.to_owned(), cwd.clone());
        info.title = title.map(str::to_owned);
        info.updated_at = updated.map(str::to_owned);
        info
    };
    match cursor {
        None => {
            let mut listed = ListSessionsResponse::new(vec![
                entry("past-3", Some("third"), Some("2026-11-03T09:00:00Z")),
                entry("past-2", Some("second"), Some("2026-11-02T09:00:00Z")),
                SessionInfo::new("past-relative", "relative/dir"),
            ]);
            listed.next_cursor = Some(SECOND_PAGE.to_owned());
            Ok(listed)
        }
        Some(SECOND_PAGE) => Ok(ListSessionsResponse::new(vec![entry("past-1", None, None)])),
        Some(other) => Err(Error::invalid_params().data(format!("unknown cursor: {other}"))),
    }
}
