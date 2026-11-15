//! Debounce, cache and stale-reply rules; no timers or node calls live here.
use super::{CACHE_QUERIES, CACHE_TTL, Mention, Problem, RESULT_LIMIT, active};
use crate::composer::drafts::DraftKey;
use crate::composer::slash::{Chosen, Step};
use crate::machine_book::Name;
use std::time::Duration;
use vessel_proto::workspace::{RelPath, WorkspaceRoot};

/// UI correlation is distinct from a wire request id or a creation ticket.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Ticket(u64);
impl Ticket {
    /// Reserve a later lookup identity without wrapping into an old reply.
    #[must_use]
    pub const fn checked_next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

/// A query belongs to one composer on one already authorized node/root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    /// The exact composer which may consume the reply.
    pub draft: DraftKey,
    /// Its validated book key.
    pub machine: Name,
    /// Existing workspace authority, not a duplicate workspace registry id.
    pub root: WorkspaceRoot,
}

/// One debounced request with its token range captured before dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Local correlation; an old result cannot overwrite another query.
    pub ticket: Ticket,
    /// The node and composer whose input produced it.
    pub scope: Scope,
    /// The validated token and query.
    pub mention: Mention,
}

/// Only file and directory candidates are selectable in the composer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionKind {
    /// File pointer.
    File,
    /// Directory pointer.
    Directory,
}

/// A node result projected into a typed selectable row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    /// A path validated at the node/file boundary.
    pub path: RelPath,
    /// The row's semantic kind and glyph.
    pub kind: SuggestionKind,
}

/// Safe UI failures; the link retains its transport/node diagnostic separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// No connected node accepted the lookup.
    Offline,
    /// The peer is too old for the lookup method.
    Unsupported,
    /// The named lookup deadline expired.
    TimedOut,
    /// The peer explicitly refused the request.
    Refused(vessel_proto::ErrorCode),
    /// A reply exceeded or violated its contract.
    InvalidReply,
}

/// Every visible suggestion state is explicit, including an empty successful lookup.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Phase {
    /// No eligible token or no workspace root.
    #[default]
    Hidden,
    /// Keep prior rows while typing settles.
    Debouncing {
        /// Pending request.
        request: Request,
        /// Last successful rows, if any.
        previous: Vec<Suggestion>,
    },
    /// A request is on the wire; previous rows remain selectable as in Paseo.
    Loading {
        /// In-flight request.
        request: Request,
        /// Last successful rows.
        previous: Vec<Suggestion>,
    },
    /// Best-first results; the renderer draws them in reverse above the input.
    Rows {
        /// Accepted request.
        request: Request,
        /// Validated rows.
        entries: Vec<Suggestion>,
    },
    /// A successful query with no candidate.
    Empty(Request),
    /// A failed request, rather than an empty result fabricated from failure.
    Failed {
        /// Failed query identity.
        request: Request,
        /// Safe recovery reason.
        reason: Failure,
    },
    /// The input itself cannot be sent to the node.
    Invalid(Problem),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Cached {
    scope: Scope,
    query: super::Query,
    rows: Vec<Suggestion>,
    at: Duration,
}

/// One composer's suggestion state and bounded short-lived cache.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Menu {
    phase: Phase,
    chosen: Chosen,
    clock: u64,
    dismissed: Option<(Scope, Mention)>,
    cache: Vec<Cached>,
}

impl Phase {
    fn request(&self) -> Option<&Request> {
        match self {
            Self::Debouncing { request, .. }
            | Self::Loading { request, .. }
            | Self::Rows { request, .. }
            | Self::Empty(request)
            | Self::Failed { request, .. } => Some(request),
            Self::Hidden | Self::Invalid(_) => None,
        }
    }
    fn rows(&self) -> &[Suggestion] {
        match self {
            Self::Rows { entries, .. } => entries,
            Self::Debouncing { previous, .. } | Self::Loading { previous, .. } => previous,
            Self::Hidden | Self::Invalid(_) | Self::Empty(_) | Self::Failed { .. } => &[],
        }
    }
}

impl Menu {
    /// Preserve the owner's checked allocation clock when presentation is evicted.
    /// Existing request identities and cached rows are not rewritten.
    pub fn inherit_ticket_clock(&mut self, clock: Ticket) {
        self.clock = self.clock.max(clock.0);
    }
    /// The owner retains only this counter across bounded presentation eviction.
    #[must_use]
    pub const fn ticket_clock(&self) -> Ticket {
        Ticket(self.clock)
    }

    /// Observe the exact current token before scheduling a debounce.
    ///
    /// # Errors
    /// Rejects an invalid query or exhausted correlation counter.
    pub fn observe(
        &mut self,
        scope: Option<Scope>,
        text: &str,
        caret: usize,
        now: Duration,
    ) -> Result<Option<Request>, Problem> {
        let Some(scope) = scope else {
            self.phase = Phase::Hidden;
            self.dismissed = None;
            return Ok(None);
        };
        let mention = match active(text, caret) {
            Ok(mention) => mention,
            Err(problem) => {
                self.phase = Phase::Invalid(problem.clone());
                return Err(problem);
            }
        };
        let Some(mention) = mention else {
            self.phase = Phase::Hidden;
            self.dismissed = None;
            return Ok(None);
        };
        if self.dismissed.as_ref() == Some(&(scope.clone(), mention.clone())) {
            return Ok(None);
        }
        if self
            .phase
            .request()
            .is_some_and(|old| old.scope == scope && old.mention == mention)
        {
            return Ok(None);
        }
        self.clock = self.clock.checked_add(1).ok_or(Problem::Exhausted)?;
        let request = Request {
            ticket: Ticket(self.clock),
            scope,
            mention,
        };
        self.chosen.reset();
        if let Some(cached) = self.cache.iter().find(|cached| {
            cached.scope == request.scope
                && cached.query == *request.mention.query()
                && now
                    .checked_sub(cached.at)
                    .is_some_and(|age| age < CACHE_TTL)
        }) {
            let rows = cached.rows.clone();
            self.set_rows(request, rows);
            return Ok(None);
        }
        let previous = if self
            .phase
            .request()
            .is_some_and(|old| old.scope == request.scope)
        {
            self.phase.rows().to_vec()
        } else {
            Vec::new()
        };
        self.phase = Phase::Debouncing {
            request: request.clone(),
            previous,
        };
        Ok(Some(request))
    }

    /// A late debounce cannot send an obsolete query.
    pub fn begin(&mut self, ticket: Ticket) -> Option<Request> {
        let Phase::Debouncing { request, previous } = &self.phase else {
            return None;
        };
        if request.ticket != ticket {
            return None;
        }
        let request = request.clone();
        let previous = previous.clone();
        self.phase = Phase::Loading {
            request: request.clone(),
            previous,
        };
        Some(request)
    }

    /// Accept only the active request, and refuse oversized replies intact.
    #[must_use]
    pub fn arrived(
        &mut self,
        ticket: Ticket,
        result: Result<Vec<Suggestion>, Failure>,
        now: Duration,
    ) -> bool {
        let request = match &self.phase {
            Phase::Loading { request, .. } | Phase::Debouncing { request, .. }
                if request.ticket == ticket =>
            {
                request.clone()
            }
            Phase::Hidden
            | Phase::Rows { .. }
            | Phase::Empty(_)
            | Phase::Failed { .. }
            | Phase::Invalid(_)
            | Phase::Loading { .. }
            | Phase::Debouncing { .. } => return false,
        };
        match result {
            Ok(rows) if rows.len() <= RESULT_LIMIT => {
                self.cache.retain(|cached| {
                    !(cached.scope == request.scope && cached.query == *request.mention.query())
                });
                self.cache.push(Cached {
                    scope: request.scope.clone(),
                    query: request.mention.query().clone(),
                    rows: rows.clone(),
                    at: now,
                });
                if self.cache.len() > CACHE_QUERIES {
                    let _oldest = self.cache.remove(0);
                }
                self.set_rows(request, rows);
            }
            Ok(_) => {
                self.phase = Phase::Failed {
                    request,
                    reason: Failure::InvalidReply,
                }
            }
            Err(reason) => self.phase = Phase::Failed { request, reason },
        }
        true
    }

    /// The renderer projects this enum onto the kit's loading/empty/error rows.
    #[must_use]
    pub fn phase(&self) -> &Phase {
        &self.phase
    }
    /// The current captured token, needed when committing a selection.
    #[must_use]
    pub fn request(&self) -> Option<&Request> {
        self.phase.request()
    }

    /// Best-first results remain separate from the above-input draw order.
    #[must_use]
    pub fn rows(&self) -> &[Suggestion] {
        self.phase.rows()
    }
    /// Drawn index chosen by a keyboard action.
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        self.chosen.taken(self.rows().len())
    }
    /// Move within the actual shown rows, including previous-data placeholders.
    pub fn step(&mut self, step: Step) -> Option<usize> {
        self.chosen.step(step, self.rows().len())
    }
    /// Resolve a clicked/drawn row without an unchecked index.
    #[must_use]
    pub fn row(&self, drawn: usize) -> Option<&Suggestion> {
        self.rows()
            .len()
            .checked_sub(drawn.saturating_add(1))
            .and_then(|index| self.rows().get(index))
    }
    /// Escape remains dismissed until the exact token or scope changes.
    pub fn dismiss(&mut self) {
        self.dismissed = self
            .phase
            .request()
            .map(|request| (request.scope.clone(), request.mention.clone()));
        self.phase = Phase::Hidden;
        self.chosen.reset();
    }

    fn set_rows(&mut self, request: Request, rows: Vec<Suggestion>) {
        let _selected = self
            .chosen
            .follow(request.mention.query().as_str(), rows.len());
        self.phase = if rows.is_empty() {
            Phase::Empty(request)
        } else {
            Phase::Rows {
                request,
                entries: rows,
            }
        };
    }
}
