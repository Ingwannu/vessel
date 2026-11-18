//! A typed UI/node boundary usable before the wire adapter is integrated.

use crate::devices::{MachineKey, Ticket};
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use vessel_proto::{
    ErrorCode, HarnessRef, HarnessSessionId, ProfileName, SessionId, SessionInfo, SessionMode,
    SessionSpec, SessionTitle,
};

/// A history belongs to one machine, agent, account and optional directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    /// Original authenticated admission, including same-node reconnect generation.
    pub context: crate::connection::ConnectionContext,
    /// Already parsed book identity.
    pub machine: MachineKey,
    /// Agent whose native history is requested.
    pub harness: HarnessRef,
    /// Account used for both listing and creation.
    pub profile: Option<ProfileName>,
    /// Absent lists every directory on this machine.
    pub cwd: Option<PathBuf>,
}

/// A cursor is opaque and cannot be interpreted as an offset by the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor(String);

impl Cursor {
    /// The node's history cursor byte bound (ADR 0053).
    pub const MAX_BYTES: usize = vessel_proto::history::GetAgentHistory::MAX_CURSOR_BYTES;
    /// Decode the node's opaque cursor without letting an unbounded reply persist.
    /// # Errors
    /// Refuses an overlong cursor.
    pub fn parse(text: String) -> Result<Self, Problem> {
        if text.len() > Self::MAX_BYTES {
            return Err(Problem::InvalidReply);
        }
        Ok(Self(text))
    }
    /// The original cursor, unchanged, for the next request.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Native agent history projected into the screen's vocabulary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Identity within this agent/account.
    pub id: HarnessSessionId,
    /// Directory to preserve on import.
    pub cwd: PathBuf,
    /// Agent title, if available.
    pub title: Option<SessionTitle>,
    /// Unknown dates stay unknown and sort after reported dates.
    pub updated_at: Option<DateTime<Utc>>,
    /// Existing session to open instead of duplicating it.
    pub in_vessel: Option<SessionId>,
    /// Its first prompt, from a ranked listing (ADR 0109); `None` from an
    /// older node or when the node did not read it in time.
    pub first_prompt: Option<String>,
    /// Its last prompt, as `first_prompt`.
    pub last_prompt: Option<String>,
}

impl Entry {
    /// Paseo's `getPromptPreview`: the last prompt, else the first; `None`
    /// when the node read neither (the row then says "No prompt preview").
    #[must_use]
    pub fn preview(&self) -> Option<&str> {
        [self.last_prompt.as_deref(), self.first_prompt.as_deref()]
            .into_iter()
            .flatten()
            .map(str::trim)
            .find(|text| !text.is_empty())
    }

    /// Paseo's `getSessionTitle`: the agent's title, else the first prompt;
    /// `None` for "Untitled session".
    #[must_use]
    pub fn shown_title(&self) -> Option<&str> {
        self.title
            .as_ref()
            .map(SessionTitle::as_str)
            .or(self.first_prompt.as_deref())
            .map(str::trim)
            .filter(|text| !text.is_empty())
    }

    /// Resume the selected native conversation, preserving its account and title.
    #[must_use]
    pub fn spec(&self, scope: &Scope) -> SessionSpec {
        let mut spec = SessionSpec::new(
            scope.harness.clone(),
            SessionMode::Structured {
                model: None,
                resume: Some(self.id.clone()),
                options: Vec::new(),
            },
        )
        .with_cwd(self.cwd.clone());
        spec.profile.clone_from(&scope.profile);
        spec.title.clone_from(&self.title);
        spec
    }
}

/// One decoded page; an empty page with a next cursor is not the end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    /// Conversations found in this page.
    pub entries: Vec<Entry>,
    /// Identity reported by the authenticated connection, not its mutable book name.
    pub node: vessel_proto::NodeId,
    /// Next page as minted by the node.
    pub next: Option<Cursor>,
    /// A ranked listing's count of conversations left out because they are
    /// already in Vessel (ADR 0109); 0 from a cursor page.
    pub already_imported: u32,
}

/// What a ranked listing asks (ADR 0109, `RANKED_HISTORY_PROTOCOL`): Paseo's
/// per-provider `limit` and its search, both answered by the node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ranked {
    /// The newest this many not yet in Vessel (Paseo's `PAGE_LIMITS`).
    pub limit: u16,
    /// What they must hold; `None` for all.
    pub query: Option<String>,
}

/// A creation batch remains bound to its original scope after navigation.
#[derive(Debug, Clone)]
pub struct Batch {
    /// The node whose history was listed; a reused book alias cannot substitute another.
    pub node: vessel_proto::NodeId,
    /// Agent/account and machine selected before importing.
    pub scope: Scope,
    /// Identity independent from listing requests.
    pub ticket: Ticket,
    /// Each entry is imported once, sequentially.
    pub entries: Vec<Entry>,
}

/// Commands have no stringly named method or unchecked JSON payload.
#[derive(Debug, Clone)]
pub enum Ask {
    /// List account choices for the selected agent.
    Profiles {
        /// Machine/agent being browsed.
        scope: Scope,
        /// Identity for rejecting old account lists.
        ticket: Ticket,
    },
    /// Read one page.
    List {
        /// Scope retained until the reply.
        scope: Scope,
        /// Request identity.
        ticket: Ticket,
        /// Absent starts a fresh list.
        cursor: Option<Cursor>,
        /// A ranked listing instead of a cursor page (never with a cursor).
        ranked: Option<Ranked>,
    },
    /// Import one or several selected conversations.
    Import(Batch),
}

/// Safe failures; raw node messages never cross into a rendering callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Problem {
    /// A book alias now connects to a different authenticated node.
    #[error("machine identity changed; refresh history before importing")]
    IdentityChanged,
    /// No connection.
    #[error("machine disconnected")]
    Offline,
    /// A queue did not accept the command.
    #[error("request was not sent")]
    Unsent,
    /// Existing call unavailable on an older peer.
    #[error("update this machine to import history")]
    Unsupported,
    /// Read timed out; retry is explicit.
    #[error("history read timed out")]
    Timeout,
    /// Create may have reached the node; refresh before any repeat.
    #[error("import outcome unknown; refresh history before trying again")]
    Uncertain,
    /// Safe node refusal code.
    #[error("machine refused the request ({0:?})")]
    Refused(ErrorCode),
    /// The node returned an invalid cursor or creation result.
    #[error("invalid history reply")]
    InvalidReply,
    /// Retained history or explicit batch exceeds this sheet's finite budget.
    #[error("narrow the history scope or import fewer conversations at once")]
    Capacity,
    /// Never reuse a request identity.
    #[error("request identities exhausted")]
    Exhausted,
}

/// One creation receipt, including the native id even after a failed create.
#[derive(Debug, Clone)]
pub struct Outcome {
    /// Which selected row produced this receipt.
    pub id: HarnessSessionId,
    /// Authoritative created session, or safe failure.
    pub result: Result<Box<SessionInfo>, Problem>,
}

/// Answers are keyed independently of UI selection and arrival order.
#[derive(Debug, Clone)]
pub enum Reply {
    /// Accounts offered by the selected machine and agent.
    Profiles {
        /// Requested machine/agent.
        scope: Scope,
        /// Request identity.
        ticket: Ticket,
        /// Safe profile metadata, never credentials.
        result: Result<Vec<vessel_proto::ProfileInfo>, Problem>,
    },
    /// One page.
    Listed {
        /// Requested scope.
        scope: Scope,
        /// Request identity.
        ticket: Ticket,
        /// Result.
        result: Result<Listed, Problem>,
    },
    /// Batch complete; no create is repeated automatically.
    Imported {
        /// Original scope.
        scope: Scope,
        /// Batch identity.
        ticket: Ticket,
        /// Ordered receipts.
        outcomes: Vec<Outcome>,
    },
}

impl Ask {
    /// Immutable source captured when the user opened this history.
    #[must_use]
    pub const fn scope(&self) -> &Scope {
        match self {
            Self::List { scope, .. } | Self::Profiles { scope, .. } => scope,
            Self::Import(batch) => &batch.scope,
        }
    }
    /// A rejected queue settles every waiting row without starting anything.
    #[must_use]
    pub fn refused(self, problem: Problem) -> Reply {
        match self {
            Self::Profiles { scope, ticket } => Reply::Profiles {
                scope,
                ticket,
                result: Err(problem),
            },
            Self::List { scope, ticket, .. } => Reply::Listed {
                scope,
                ticket,
                result: Err(problem),
            },
            Self::Import(batch) => Reply::Imported {
                scope: batch.scope,
                ticket: batch.ticket,
                outcomes: batch
                    .entries
                    .into_iter()
                    .map(|entry| Outcome {
                        id: entry.id,
                        result: Err(problem),
                    })
                    .collect(),
            },
        }
    }
}

impl Reply {
    /// Original scope for immediate receiver-side delivery validation.
    #[must_use]
    pub const fn scope(&self) -> &Scope {
        match self {
            Self::Profiles { scope, .. }
            | Self::Listed { scope, .. }
            | Self::Imported { scope, .. } => scope,
        }
    }
    /// Invalidated observations cannot install data; any already-issued import
    /// remains uncertain even if its successful receipt had reached the UI queue.
    #[must_use]
    pub fn invalidated(self, read_problem: Problem) -> Self {
        match self {
            Self::Profiles { scope, ticket, .. } => Self::Profiles {
                scope,
                ticket,
                result: Err(read_problem),
            },
            Self::Listed { scope, ticket, .. } => Self::Listed {
                scope,
                ticket,
                result: Err(read_problem),
            },
            Self::Imported {
                scope,
                ticket,
                outcomes,
            } => Self::Imported {
                scope,
                ticket,
                outcomes: outcomes
                    .into_iter()
                    .map(|outcome| Outcome {
                        id: outcome.id,
                        result: Err(Problem::Uncertain),
                    })
                    .collect(),
            },
        }
    }
}
