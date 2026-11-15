//! Page identity and explicit imports keep late replies from changing scope.

use super::{Ask, Batch, Cursor, Entry, Listed, Outcome, Problem, Reply, Scope};
use crate::devices::{Ticket, Tickets};
use crate::matcher::Query;
use vessel_proto::{HarnessSessionId, SessionId, SessionTitle};

/// Reads are independent of retained rows, so retry never erases good data.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum Loading {
    /// No read is pending.
    #[default]
    Idle,
    /// The exact outstanding page request.
    Pending(Ticket),
    /// Read refused; rows already fetched remain available.
    Failed(Problem),
}

/// Selecting rows is explicit; existing sessions are opened, not selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    /// Selected for the next import.
    Selected,
    /// Available but not selected.
    Available,
    /// Already imported on this node.
    Existing(SessionId),
}

#[derive(Debug)]
struct PendingImport {
    scope: Scope,
    ticket: Ticket,
    ids: Vec<HarnessSessionId>,
}

/// A sheet's pure state; its ticket source survives closing and reopening.
#[derive(Debug, Default)]
pub struct History {
    scope: Option<Scope>,
    node: Option<vessel_proto::NodeId>,
    entries: Vec<Entry>,
    next: Option<Cursor>,
    loading: Loading,
    selected: Vec<HarnessSessionId>,
    importing: Option<PendingImport>,
    outcomes: Vec<Outcome>,
    tickets: Tickets,
}

impl History {
    /// Read current scope without copying a directory or account.
    #[must_use]
    pub fn scope(&self) -> Option<&Scope> {
        self.scope.as_ref()
    }
    /// Authenticated identity pinned by the history page, not the live book alias.
    #[must_use]
    pub const fn listed_node(&self) -> Option<vessel_proto::NodeId> {
        self.node
    }
    /// Rows are sorted once when a page arrives, not in rendering.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
    /// Failed batches retain individual receipts beside their rows.
    #[must_use]
    pub fn outcomes(&self) -> &[Outcome] {
        &self.outcomes
    }
    /// State for a spinner or inline read failure.
    #[must_use]
    pub fn loading(&self) -> &Loading {
        &self.loading
    }
    /// Whether the node reports another page, including an empty filtered page.
    #[must_use]
    pub fn has_more(&self) -> bool {
        self.next.is_some()
    }
    /// A batch locks selection and scope until its receipts arrive.
    #[must_use]
    pub fn importing(&self) -> bool {
        self.importing.is_some()
    }
    /// Whether an import button has something to submit.
    #[must_use]
    pub fn has_selection(&self) -> bool {
        !self.selected.is_empty()
    }

    /// Only loaded rows are searched; opaque future pages are never claimed.
    #[must_use]
    pub fn filtered(&self, query: &Query) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|entry| {
                let path = entry.cwd.to_string_lossy();
                let title = entry.title.as_ref().map_or("", SessionTitle::as_str);
                query.is_empty() || query.score(&[title, &path], None).is_some()
            })
            .collect()
    }

    /// An ambiguous creation must be reconciled by a fresh read before retry.
    #[must_use]
    pub fn requires_refresh(&self) -> bool {
        self.outcomes.iter().any(|outcome| {
            matches!(
                outcome.result,
                Err(Problem::Uncertain | Problem::InvalidReply)
            )
        })
    }

    /// Closing invalidates listings while retaining in-flight creation receipts.
    pub fn close(&mut self) {
        self.scope = None;
        self.node = None;
        self.loading = Loading::Idle;
        self.entries.clear();
        self.next = None;
        self.selected.clear();
    }

    /// A different machine/account/folder starts with no borrowed rows.
    pub fn choose(&mut self, scope: Scope) {
        if self.scope.as_ref() != Some(&scope) && !self.importing() {
            self.close();
            self.outcomes.clear();
            self.scope = Some(scope);
        }
    }

    /// Read from the beginning, keeping current rows until a successful answer.
    /// # Errors
    /// Ticket space is exhausted; no request was issued.
    pub fn refresh(&mut self) -> Result<Option<Ask>, Problem> {
        self.request(None)
    }

    /// The next opaque page; no call when none exists or a read is pending.
    /// # Errors
    /// Ticket space is exhausted.
    pub fn more(&mut self) -> Result<Option<Ask>, Problem> {
        match self.next.clone() {
            Some(cursor) => self.request(Some(cursor)),
            None => Ok(None),
        }
    }

    fn request(&mut self, cursor: Option<Cursor>) -> Result<Option<Ask>, Problem> {
        if matches!(self.loading, Loading::Pending(_)) || self.importing() {
            return Ok(None);
        }
        let Some(scope) = self.scope.clone() else {
            return Ok(None);
        };
        // Tickets only returns the fieldless Exhausted variant; preserve its meaning.
        let ticket = self
            .tickets
            .next_ticket()
            .map_err(|_exhausted| Problem::Exhausted)?;
        self.loading = Loading::Pending(ticket);
        // A fresh request only replaces rows after success; the answer remembers
        // whether it was a continuation through the cursor retained here.
        if cursor.is_none() {
            self.next = None;
        }
        Ok(Some(Ask::List {
            scope,
            ticket,
            cursor,
        }))
    }

    /// Resolve a row through its native id, never its screen index.
    #[must_use]
    pub fn selection(&self, id: &HarnessSessionId) -> Selection {
        if let Some(session) = self
            .entries
            .iter()
            .find(|entry| &entry.id == id)
            .and_then(|entry| entry.in_vessel)
        {
            Selection::Existing(session)
        } else if self.selected.contains(id) {
            Selection::Selected
        } else {
            Selection::Available
        }
    }

    /// Toggle only a currently available row; importing freezes its selection.
    pub fn toggle(&mut self, id: &HarnessSessionId) {
        if self.importing() || !self.entries.iter().any(|entry| &entry.id == id) {
            return;
        }
        match self.selection(id) {
            Selection::Existing(_) => {}
            Selection::Selected => self.selected.retain(|selected| selected != id),
            Selection::Available => self.selected.push(id.clone()),
        }
    }

    /// Prepare every create from the listed row's own directory and account.
    /// # Errors
    /// Ticket identities are exhausted.
    pub fn import(&mut self) -> Result<Option<Ask>, Problem> {
        if self.importing()
            || self.requires_refresh()
            || matches!(self.loading, Loading::Pending(_))
        {
            return Ok(None);
        }
        let entries: Vec<_> = self
            .entries
            .iter()
            .filter(|entry| entry.in_vessel.is_none() && self.selected.contains(&entry.id))
            .cloned()
            .collect();
        self.begin(entries)
    }

    /// Open an existing native conversation by reactivating its original session.
    /// # Errors
    /// Ticket identities are exhausted; no request was issued.
    pub fn activate(&mut self, id: &HarnessSessionId) -> Result<Option<Ask>, Problem> {
        if self.importing()
            || self.requires_refresh()
            || matches!(self.loading, Loading::Pending(_))
        {
            return Ok(None);
        }
        let entries = self
            .entries
            .iter()
            .find(|entry| &entry.id == id && entry.in_vessel.is_some())
            .cloned()
            .into_iter()
            .collect();
        self.begin(entries)
    }

    fn begin(&mut self, entries: Vec<Entry>) -> Result<Option<Ask>, Problem> {
        let Some(scope) = self.scope.clone() else {
            return Ok(None);
        };
        let Some(node) = self.node else {
            return Ok(None);
        };
        if entries.is_empty() {
            return Ok(None);
        }
        if entries.len() > super::MAX_IMPORT_ENTRIES {
            return Err(Problem::Capacity);
        }
        // Tickets only returns the fieldless Exhausted variant; preserve its meaning.
        let ticket = self
            .tickets
            .next_ticket()
            .map_err(|_exhausted| Problem::Exhausted)?;
        self.importing = Some(PendingImport {
            scope: scope.clone(),
            ticket,
            ids: entries.iter().map(|entry| entry.id.clone()).collect(),
        });
        self.outcomes.clear();
        Ok(Some(Ask::Import(Batch {
            node,
            scope,
            ticket,
            entries,
        })))
    }

    /// Accept only the outstanding scope/ticket, returning whether it applied.
    #[must_use]
    pub fn answer(&mut self, reply: Reply) -> bool {
        match reply {
            Reply::Profiles { .. } => false,
            Reply::Listed {
                scope,
                ticket,
                result,
            } => self.listed(&scope, ticket, result),
            Reply::Imported {
                scope,
                ticket,
                outcomes,
            } => self.imported(&scope, ticket, outcomes),
        }
    }

    fn listed(&mut self, scope: &Scope, ticket: Ticket, result: Result<Listed, Problem>) -> bool {
        if self.scope.as_ref() != Some(scope) || self.loading != Loading::Pending(ticket) {
            return false;
        }
        match result {
            Ok(page) if self.next.is_some() && self.node.is_some_and(|node| node != page.node) => {
                self.loading = Loading::Failed(Problem::IdentityChanged);
            }
            Ok(page) => {
                let retained = if self.next.is_some() {
                    self.entries.as_slice()
                } else {
                    &[]
                };
                if let Err(problem) = super::bounds::admit(retained, &page.entries) {
                    self.loading = Loading::Failed(problem);
                    return true;
                }
                self.node = Some(page.node);
                self.page(page);
                self.loading = Loading::Idle;
            }
            Err(problem) => self.loading = Loading::Failed(problem),
        }
        true
    }

    fn imported(&mut self, scope: &Scope, ticket: Ticket, outcomes: Vec<Outcome>) -> bool {
        let Some(pending) = &self.importing else {
            return false;
        };
        if pending.ticket != ticket || &pending.scope != scope {
            return false;
        }
        let complete = outcomes.len() == pending.ids.len()
            && pending
                .ids
                .iter()
                .all(|id| outcomes.iter().filter(|outcome| &outcome.id == id).count() == 1);
        let outcomes = if complete {
            outcomes
        } else {
            pending
                .ids
                .iter()
                .cloned()
                .map(|id| Outcome {
                    id,
                    result: Err(Problem::InvalidReply),
                })
                .collect()
        };
        self.apply_receipts(outcomes);
        true
    }

    fn apply_receipts(&mut self, outcomes: Vec<Outcome>) {
        for receipt in &outcomes {
            if let Ok(info) = &receipt.result
                && let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == receipt.id)
            {
                entry.in_vessel = Some(info.id);
            }
        }
        self.outcomes = outcomes;
        self.selected.clear();
        self.importing = None;
    }

    fn page(&mut self, page: Listed) {
        if self.next.is_none() {
            self.entries.clear();
            self.outcomes.clear();
        }
        for entry in page.entries {
            if let Some(existing) = self.entries.iter_mut().find(|old| old.id == entry.id) {
                *existing = entry;
            } else {
                self.entries.push(entry);
            }
        }
        self.entries.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        self.selected.retain(|id| {
            self.entries
                .iter()
                .any(|entry| &entry.id == id && entry.in_vessel.is_none())
        });
        self.next = page.next;
    }
}
