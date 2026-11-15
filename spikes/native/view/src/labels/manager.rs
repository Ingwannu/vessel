//! Manager transitions preserve drafts on failure and inspect before deletion.

use super::{Catalog, Count, Failure, HostKey, HostState, Label, Name, Ticket};

/// Uncommitted user input. Only `replacement` can cross into a node request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    original: Label,
    text: String,
    color: super::Color,
}

impl Draft {
    /// Begin from the host's current definition.
    #[must_use]
    pub fn new(label: Label) -> Self {
        Self {
            text: label.name.as_str().to_owned(),
            color: label.color,
            original: label,
        }
    }

    /// The immutable target, even while the user types a replacement name.
    #[must_use]
    pub const fn original(&self) -> &Label {
        &self.original
    }

    /// Text as typed, including an invalid empty draft.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Chosen swatch.
    #[must_use]
    pub const fn color(&self) -> super::Color {
        self.color
    }

    /// Keep draft input without claiming it is a valid label.
    pub fn set_text(&mut self, text: &str) {
        text.clone_into(&mut self.text);
    }

    /// Choosing a swatch does not save the form.
    pub const fn set_color(&mut self, color: super::Color) {
        self.color = color;
    }

    /// No-change and whitespace-only drafts do not enable Save.
    #[must_use]
    pub fn replacement(&self, current: &Label) -> Option<Label> {
        let name = self.text.parse::<Name>().ok()?;
        let label = Label {
            name,
            color: self.color,
        };
        (label != *current).then_some(label)
    }
}

/// Every asynchronous phase carries the exact draft and correlation it owns.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum ManagerState {
    /// Searchable catalog.
    #[default]
    Listing,
    /// An edit that has not been sent.
    Editing(Draft),
    /// Atomic rename/recolor in flight.
    Saving {
        /// Request identity.
        ticket: Ticket,
        /// Preserved edit.
        draft: Draft,
    },
    /// Fetch the advisory affected-workspace count before confirmation.
    Inspecting {
        /// Request identity.
        ticket: Ticket,
        /// Preserved edit.
        draft: Draft,
    },
    /// Only this state may submit Delete.
    Confirming {
        /// Inspection identity.
        ticket: Ticket,
        /// Preserved edit.
        draft: Draft,
        /// Advisory affected workspaces.
        count: Count,
    },
    /// Confirmed deletion in flight.
    Deleting {
        /// Request identity.
        ticket: Ticket,
        /// Preserved edit.
        draft: Draft,
    },
}

impl ManagerState {
    /// No input or host switching while a mutation owns the surface.
    #[must_use]
    pub const fn busy(&self) -> bool {
        match self {
            Self::Listing | Self::Editing(_) => false,
            Self::Saving { .. }
            | Self::Inspecting { .. }
            | Self::Confirming { .. }
            | Self::Deleting { .. } => true,
        }
    }

    /// Draw the same draft through every asynchronous phase.
    #[must_use]
    pub const fn draft(&self) -> Option<&Draft> {
        match self {
            Self::Listing => None,
            Self::Editing(draft)
            | Self::Saving { draft, .. }
            | Self::Inspecting { draft, .. }
            | Self::Confirming { draft, .. }
            | Self::Deleting { draft, .. } => Some(draft),
        }
    }
}

/// Host selection and form state; catalogs are stored separately by their host.
#[derive(Debug, Default)]
pub struct Manager {
    selected: Option<HostKey>,
    state: ManagerState,
    query: String,
    error: Option<Failure>,
}

impl Manager {
    /// A removed host resets idle drafts; a pending operation keeps its owner.
    pub fn sync_hosts(&mut self, hosts: &[super::Host]) {
        if self.state.busy() {
            return;
        }
        if !hosts.iter().any(|host| Some(host.key) == self.selected) {
            self.selected = hosts.first().map(|host| host.key);
            self.state = ManagerState::Listing;
            self.error = None;
        }
    }
    /// Name/color input stays locked across every asynchronous phase.
    #[must_use]
    pub fn edit_access(&self, host: HostState) -> super::EditAccess {
        if host.connected() && matches!(self.state, ManagerState::Editing(_)) {
            super::EditAccess::Editable
        } else {
            super::EditAccess::ReadOnly
        }
    }
    /// Current phase for rendering.
    #[must_use]
    pub const fn state(&self) -> &ManagerState {
        &self.state
    }
    /// Selected host, retained while an operation is pending.
    #[must_use]
    pub const fn selected(&self) -> Option<&HostKey> {
        self.selected.as_ref()
    }
    /// Search input retained across entering and leaving an edit.
    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }
    /// Search uses the same reference edge whitespace as the picker.
    #[must_use]
    pub fn filtered<'a>(&self, catalog: &'a Catalog) -> Vec<&'a Label> {
        let query = super::search::Query::new(&self.query);
        catalog
            .labels()
            .filter(|label| query.matches(&label.name))
            .collect()
    }
    /// Empty-after-trim search has the catalog's empty state, not no-matches.
    #[must_use]
    pub fn has_query(&self) -> bool {
        !super::search::Query::new(&self.query).is_empty()
    }
    /// Last safe operation error; draft remains intact.
    #[must_use]
    pub const fn error(&self) -> Option<Failure> {
        self.error
    }
    /// Search changes presentation, never catalog membership.
    pub fn search(&mut self, query: &str) {
        query.clone_into(&mut self.query);
    }

    /// Switch to a host and discard the old draft only while idle.
    /// # Errors
    /// Refuses switching during an operation or confirmation.
    pub fn select(&mut self, host: HostKey) -> Result<(), Failure> {
        if self.state.busy() {
            return Err(Failure::Pending);
        }
        if self.selected.as_ref() != Some(&host) {
            self.selected = Some(host);
            self.state = ManagerState::Listing;
            self.error = None;
        }
        Ok(())
    }

    /// Enter editing without changing the authoritative catalog.
    /// # Errors
    /// Refuses replacing a pending operation's draft.
    pub fn edit(&mut self, label: Label) -> Result<(), Failure> {
        if self.state.busy() {
            return Err(Failure::Pending);
        }
        self.state = ManagerState::Editing(Draft::new(label));
        self.error = None;
        Ok(())
    }

    /// Only an idle editor accepts input; in-flight payloads stay immutable.
    pub fn draft_mut(&mut self) -> Option<&mut Draft> {
        match &mut self.state {
            ManagerState::Editing(draft) => Some(draft),
            ManagerState::Listing
            | ManagerState::Saving { .. }
            | ManagerState::Inspecting { .. }
            | ManagerState::Confirming { .. }
            | ManagerState::Deleting { .. } => None,
        }
    }

    /// Closing resets search and drafts so reopening has no stale input.
    /// # Errors
    /// A pending operation must settle before its surface is reset.
    pub fn reset(&mut self) -> Result<(), Failure> {
        if self.state.busy() {
            return Err(Failure::Pending);
        }
        self.state = ManagerState::Listing;
        self.query.clear();
        self.error = None;
        Ok(())
    }

    /// Back returns to the searchable list without losing its query.
    /// # Errors
    /// A pending operation or confirmation must settle first.
    pub fn back(&mut self) -> Result<(), Failure> {
        if self.state.busy() {
            return Err(Failure::Pending);
        }
        self.state = ManagerState::Listing;
        self.error = None;
        Ok(())
    }

    /// Validate and reserve one atomic rename/recolor request.
    /// # Errors
    /// Refuses unavailable hosts, pending work and empty/unchanged drafts.
    pub fn save(
        &mut self,
        host: HostState,
        catalog: &Catalog,
        ticket: Ticket,
    ) -> Result<Label, Failure> {
        host.writable()?;
        let draft = self.editing()?;
        let current = catalog.get(&draft.original.name).ok_or(Failure::NotFound)?;
        let replacement = draft.replacement(current).ok_or(Failure::NoChange)?;
        self.state = ManagerState::Saving { ticket, draft };
        self.error = None;
        Ok(replacement)
    }

    /// Inspection is mandatory before a destructive confirmation is presented.
    /// # Errors
    /// Refuses unavailable hosts and non-editing phases.
    pub fn inspect(
        &mut self,
        host: HostState,
        catalog: &Catalog,
        ticket: Ticket,
    ) -> Result<Name, Failure> {
        host.writable()?;
        let draft = self.editing()?;
        let name = catalog
            .get(&draft.original.name)
            .ok_or(Failure::NotFound)?
            .name
            .clone();
        self.state = ManagerState::Inspecting { ticket, draft };
        self.error = None;
        Ok(name)
    }

    /// Reject stale inspection answers instead of confirming the wrong label.
    #[must_use]
    pub fn inspected(&mut self, ticket: Ticket, result: Result<Count, Failure>) -> bool {
        let ManagerState::Inspecting {
            ticket: pending,
            draft,
        } = &self.state
        else {
            return false;
        };
        if *pending != ticket {
            return false;
        }
        let draft = draft.clone();
        self.state = match result {
            Ok(count) => ManagerState::Confirming {
                ticket,
                draft,
                count,
            },
            Err(error) => {
                self.error = Some(error);
                ManagerState::Editing(draft)
            }
        };
        true
    }

    /// Cancel confirmation returns to the draft; it never sends a deletion.
    #[must_use]
    pub fn cancel_delete(&mut self) -> bool {
        let ManagerState::Confirming { draft, .. } = &self.state else {
            return false;
        };
        self.state = ManagerState::Editing(draft.clone());
        true
    }

    /// Explicit confirmation allocates a fresh deletion ticket.
    /// # Errors
    /// Requires a successful inspection, and rechecks host availability.
    pub fn delete(&mut self, host: HostState, ticket: Ticket) -> Result<Name, Failure> {
        host.writable()?;
        let ManagerState::Confirming { draft, .. } = &self.state else {
            return Err(Failure::NoChange);
        };
        let draft = draft.clone();
        let name = draft.original.name.clone();
        self.state = ManagerState::Deleting { ticket, draft };
        Ok(name)
    }

    /// Successful mutation returns to list; failure preserves exact typed input.
    #[must_use]
    pub fn completed(&mut self, ticket: Ticket, result: Result<(), Failure>) -> bool {
        let (pending, draft) = match &self.state {
            ManagerState::Saving { ticket, draft } | ManagerState::Deleting { ticket, draft } => {
                (*ticket, draft)
            }
            ManagerState::Listing
            | ManagerState::Editing(_)
            | ManagerState::Inspecting { .. }
            | ManagerState::Confirming { .. } => return false,
        };
        if pending != ticket {
            return false;
        }
        self.state = match result {
            Ok(()) => ManagerState::Listing,
            Err(error) => {
                self.error = Some(error);
                ManagerState::Editing(draft.clone())
            }
        };
        true
    }

    fn editing(&self) -> Result<Draft, Failure> {
        match &self.state {
            ManagerState::Editing(draft) => Ok(draft.clone()),
            ManagerState::Listing => Err(Failure::NoChange),
            ManagerState::Saving { .. }
            | ManagerState::Inspecting { .. }
            | ManagerState::Confirming { .. }
            | ManagerState::Deleting { .. } => Err(Failure::Pending),
        }
    }
}
