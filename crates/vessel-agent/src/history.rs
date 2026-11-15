//! [`history`]: one page of an agent's own conversations, as it lists them
//! over its selected native protocol or ACP (`session/list`; protocol 17's
//! `agent.history`, ADR 0053).
//!
//! [Decision Log]
//! - Purpose: a person imports a conversation they had with an agent
//!   outside Vessel; only the agent knows which it has.
//! - Alternatives: reading each harness's files (agent-orchestrator and
//!   GOAT keep a reader per harness; each breaks when a harness changes its
//!   format, and reads another program's files).
//! - Choice: Paseo's ACP path: start the agent as a probe does, `initialize`,
//!   then `session/list`. No session is created or loaded, so nothing is
//!   written to the agent's history by listing it. Bounded by the handshake
//!   timeout; the agent is stopped after.
//! - Pages: an agent's page is given whole up to [`MAX_ENTRIES`] (Claude's
//!   adapter answers 1,000 a page); a larger one is given in parts, the
//!   cursor ([`HistoryFrom`]) naming the agent's page and how many of it
//!   were given. An agent that filters by folder after paging (Codex's
//!   adapter) can answer a page with nothing in it and a next one: the
//!   listing asks on, on the same connection, up to [`MAX_PAGES`] pages, as
//!   Paseo does, so an empty page is the end unless it says otherwise.
//! - Over ACP, only an agent that advertises `sessionCapabilities.list` and
//!   `loadSession` lists: a conversation that cannot be loaded cannot be
//!   imported.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::v1::schema as acp;
use agent_client_protocol::{Client, Error, ErrorCode};
use tokio::io::AsyncBufReadExt as _;
use vessel_proc::{CommandSpec, Signal};
use vessel_proto::HarnessSessionId;

use crate::drivers::Driver;
use crate::error::AgentError;
use crate::transport::{self, Gate};

/// How long the agent may take to exit once the page is read, before its
/// process group is killed.
const STOP_GRACE: Duration = Duration::from_secs(2);

/// The most entries one listing gives.
pub const MAX_ENTRIES: usize = 1000;

/// The most pages of the agent's one listing asks while they hold nothing.
pub(crate) const MAX_PAGES: usize = 10;

/// How many of the agent's last stderr lines a failure carries.
const LOG_LINES: usize = 5;

/// Provider time before the node's calendar conversion. Keeping the source format
/// avoids a second calendar dependency or a fabricated timestamp in a driver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryTimestamp {
    /// ACP's original RFC 3339 value; invalid dates stay unknown at projection.
    Rfc3339(String),
    /// Native Codex's observed Unix seconds.
    UnixSeconds(i64),
}

/// One conversation the agent keeps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    /// The agent's id for it (what `session/load` takes).
    pub session: HarnessSessionId,
    /// The folder it ran in (absolute).
    pub cwd: PathBuf,
    /// Its title, as the agent gives it.
    pub title: Option<String>,
    /// When it was last active, in the provider's observed format.
    pub updated_at: Option<HistoryTimestamp>,
}

/// A page of conversations and where the next one starts.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HistoryPage {
    /// Newest first, as the agent lists them.
    pub entries: Vec<HistoryEntry>,
    /// Where the next page starts; `None` on the last.
    pub next: Option<HistoryFrom>,
}

/// Where a listing starts: the agent's page (its cursor; the first page
/// without) and how many of that page's entries were already given.
///
/// As text (a client's opaque cursor): `<skip>~` for the agent's first
/// page, `<skip>:<agent cursor>` otherwise.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HistoryFrom {
    /// The agent's cursor for its page.
    pub cursor: Option<String>,
    /// Entries of that page already given.
    pub skip: usize,
}

impl std::fmt::Display for HistoryFrom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.cursor {
            None => write!(f, "{}~", self.skip),
            Some(cursor) => write!(f, "{}:{cursor}", self.skip),
        }
    }
}

/// A cursor no listing gave.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("not a cursor of a listing")]
pub struct NotACursor;

impl std::str::FromStr for HistoryFrom {
    type Err = NotACursor;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let digits = text.bytes().take_while(u8::is_ascii_digit).count();
        let (skip, rest) = text.split_at(digits);
        let skip = skip.parse().map_err(|_not_a_number| NotACursor)?;
        let cursor = match rest.split_at_checked(1) {
            Some(("~", "")) => None,
            Some((":", cursor)) => Some(cursor.to_owned()),
            _ => return Err(NotACursor),
        };
        Ok(Self { cursor, skip })
    }
}

/// Why no page was read.
#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    /// The agent could not be started.
    #[error("cannot start the agent")]
    Spawn(#[source] AgentError),
    /// The native listing method is absent, or ACP does not advertise listing
    /// and loading conversations.
    #[error("the agent does not list its conversations over this protocol")]
    CannotList,
    /// The agent refused the request as invalid (`invalid_params`): a
    /// cursor it no longer knows, or a folder it does not take.
    #[error("the agent refused the listing: {0}")]
    Refused(String),
    /// The connection failed, or the agent refused.
    #[error("{message}{}", Said(log))]
    Failed {
        /// What went wrong.
        message: String,
        /// Its last lines on stderr, oldest first.
        log: Vec<String>,
    },
    /// It did not answer within the limit.
    #[error("the agent did not answer within {} s{}", limit.as_secs(), Said(log))]
    TimedOut {
        /// The limit.
        limit: Duration,
        /// Its last lines on stderr, oldest first.
        log: Vec<String>,
    },
}

/// `; the agent said: a | b`, or nothing.
struct Said<'a>(&'a [String]);

impl std::fmt::Display for Said<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            return Ok(());
        }
        f.write_str("; the agent said: ")?;
        f.write_str(&self.0.join(" | "))
    }
}

/// How the exchange ended, before the log is attached.
enum Unlisted {
    CannotList,
    Refused(String),
    Failed(String),
}

/// Start `command`, ask its conversations in `cwd` (all of them without)
/// from `from`, and stop it. Bounded by `limit` plus twice
/// [`STOP_GRACE`].
///
/// # Errors
/// [`HistoryError::Spawn`], [`HistoryError::CannotList`],
/// [`HistoryError::Refused`], [`HistoryError::Failed`] or
/// [`HistoryError::TimedOut`].
pub async fn history(
    command: &CommandSpec,
    cwd: Option<&Path>,
    from: HistoryFrom,
    limit: Duration,
) -> Result<HistoryPage, HistoryError> {
    history_with(Driver::Acp, command, cwd, from, limit).await
}

/// [`history`] over the protocol `driver` speaks (ADR 0057): the same page
/// and cursor whichever driver lists.
///
/// # Errors
/// As [`history`]; [`HistoryError::Spawn`] holds
/// [`AgentError::NoDriver`](crate::AgentError::NoDriver) for a driver this
/// build does not run.
pub async fn history_with(
    driver: Driver,
    command: &CommandSpec,
    cwd: Option<&Path>,
    from: HistoryFrom,
    limit: Duration,
) -> Result<HistoryPage, HistoryError> {
    if !driver.built() {
        return Err(HistoryError::Spawn(AgentError::NoDriver(driver)));
    }
    match driver {
        Driver::Acp => over_acp(command, cwd, from, limit).await,
        Driver::CodexAppServer => crate::codex::history(command, cwd, from, limit).await,
        Driver::ClaudeStream => crate::claude::history(command, cwd, from, limit).await,
        Driver::OpencodeHttp => crate::opencode::history(command, cwd, from, limit).await,
    }
}

/// [`history`] over ACP: `session/list`.
async fn over_acp(
    command: &CommandSpec,
    cwd: Option<&Path>,
    from: HistoryFrom,
    limit: Duration,
) -> Result<HistoryPage, HistoryError> {
    let vessel_proc::Spawned {
        mut child,
        stdin,
        stdout,
        stderr,
    } = vessel_proc::spawn(command).map_err(|error| HistoryError::Spawn(error.into()))?;
    let log = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let kept = Arc::clone(&log);
    drop(tokio::spawn(async move {
        let mut lines = tokio::io::BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let mut log = kept
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if log.len() == LOG_LINES {
                let _oldest = log.remove(0);
            }
            log.push(line);
        }
    }));
    let gate = Gate::new();
    let released = gate.clone();
    let transport = transport::lines(stdin, stdout, gate);
    let cwd = cwd.map(Path::to_path_buf);
    let exchange = Client
        .v1()
        .name("vessel")
        .on_receive_notification(
            async move |_notification: acp::SessionNotification, _cx| {
                // Nothing to show while listing; the permit goes back.
                released.release();
                Ok::<(), Error>(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_with(transport, async |cx| Ok(list(&cx, cwd, from).await));
    let outcome = tokio::time::timeout(limit, exchange).await;
    stop(&mut child).await;
    let log = std::mem::take(
        &mut *log
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    );
    match outcome {
        Err(_elapsed) => Err(HistoryError::TimedOut { limit, log }),
        Ok(Err(error)) => Err(HistoryError::Failed {
            message: error.to_string(),
            log,
        }),
        Ok(Ok(Ok(page))) => Ok(page),
        Ok(Ok(Err(Unlisted::CannotList))) => Err(HistoryError::CannotList),
        Ok(Ok(Err(Unlisted::Refused(message)))) => Err(HistoryError::Refused(message)),
        Ok(Ok(Err(Unlisted::Failed(message)))) => Err(HistoryError::Failed { message, log }),
    }
}

/// `initialize`, then `session/list` from `from` (see the module's
/// Decision Log for the pages).
async fn list(
    cx: &agent_client_protocol::v1::ConnectionTo<agent_client_protocol::Agent>,
    cwd: Option<PathBuf>,
    from: HistoryFrom,
) -> Result<HistoryPage, Unlisted> {
    let initialized = cx
        .send_request(
            acp::InitializeRequest::new(ProtocolVersion::V1).client_info(acp::Implementation::new(
                "vessel",
                env!("CARGO_PKG_VERSION"),
            )),
        )
        .block_task()
        .await
        .map_err(|error| Unlisted::Failed(format!("`initialize` failed: {error}")))?;
    let capabilities = &initialized.agent_capabilities;
    if capabilities.session_capabilities.list.is_none() || !capabilities.load_session {
        return Err(Unlisted::CannotList);
    }
    let HistoryFrom {
        mut cursor,
        mut skip,
    } = from;
    let mut asked: usize = 0;
    loop {
        asked = asked.saturating_add(1);
        let listed = cx
            .send_request(
                acp::ListSessionsRequest::new()
                    .cwd(cwd.clone())
                    .cursor(cursor.clone()),
            )
            .block_task()
            .await
            .map_err(|error| {
                // An agent that advertised the listing but does not know
                // the method lists nothing either.
                if error.code == ErrorCode::METHOD_NOT_FOUND {
                    Unlisted::CannotList
                } else if error.code == ErrorCode::INVALID_PARAMS {
                    Unlisted::Refused(error.to_string())
                } else {
                    Unlisted::Failed(format!("`session/list` failed: {error}"))
                }
            })?;
        let mut entries: Vec<HistoryEntry> = listed
            .sessions
            .into_iter()
            .filter(|session| session.cwd.is_absolute())
            .skip(skip)
            .take(MAX_ENTRIES.saturating_add(1))
            .map(|session| HistoryEntry {
                session: HarnessSessionId::new(session.session_id.0),
                cwd: session.cwd,
                title: session.title,
                updated_at: session.updated_at.map(HistoryTimestamp::Rfc3339),
            })
            .collect();
        if entries.len() > MAX_ENTRIES {
            // More of this page than one listing gives: the rest of it next.
            entries.truncate(MAX_ENTRIES);
            return Ok(HistoryPage {
                entries,
                next: Some(HistoryFrom {
                    cursor,
                    skip: skip.saturating_add(MAX_ENTRIES),
                }),
            });
        }
        let next = listed.next_cursor.map(|cursor| HistoryFrom {
            cursor: Some(cursor),
            skip: 0,
        });
        match next {
            Some(next) if entries.is_empty() && asked < MAX_PAGES => {
                cursor = next.cursor;
                skip = next.skip;
            }
            next => return Ok(HistoryPage { entries, next }),
        }
    }
}

/// Ask the agent to exit, then kill its group after [`STOP_GRACE`].
async fn stop(child: &mut vessel_proc::Child) {
    if let Err(error) = child.signal(Signal::Terminate) {
        tracing::debug!(pid = child.pid(), %error, "cannot ask the listing agent to stop");
    }
    if tokio::time::timeout(STOP_GRACE, child.wait())
        .await
        .is_err()
    {
        if let Err(error) = child.signal(Signal::Kill) {
            tracing::warn!(pid = child.pid(), %error, "cannot kill the listing agent");
        }
        let _code = child.wait().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cursor_reads_back_as_it_was_written() {
        for from in [
            HistoryFrom::default(),
            HistoryFrom {
                cursor: Some("offset:1000".to_owned()),
                skip: 0,
            },
            HistoryFrom {
                cursor: Some(String::new()),
                skip: 1000,
            },
            HistoryFrom {
                cursor: None,
                skip: 2000,
            },
        ] {
            assert_eq!(from.to_string().parse::<HistoryFrom>(), Ok(from));
        }
        assert_eq!(HistoryFrom::default().to_string(), "0~");
        for not in [
            "",
            "~",
            ":x",
            "12",
            "12~x",
            "12x",
            "-1~",
            "99999999999999999999999~",
        ] {
            assert_eq!(not.parse::<HistoryFrom>(), Err(NotACursor), "{not}");
        }
    }
}
