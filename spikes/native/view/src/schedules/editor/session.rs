//! One form visit, separate from the list and from widget identity. Every
//! activation checks the visit; transport tickets share one nonreusing issuer.
//!
//! [Decision Log]
//! - Purpose: host switches/reopen cannot install old choices or repeat saves.
//! - Existing assets: Draft, typed io Mailbox, canonical `ConnectionContext`.
//! - Choice: Closed/`ChoosingHost`/Active; Active owns four operation slots under
//!   the existing actor's shared four-permit limit. No resolver/counter is added.
//! - Why/cost: pure transition guards precede every UI dispatch. A field still
//!   needs W4's source `FieldId` to carry its original visit through its callback.
use super::{definition::Draft, io};
use crate::connection::ConnectionContext;
use crate::schedules::authority::Authority;
use crate::schedules::load::{Failure, Ticket, Tickets};
use vessel_proto::automation::{Cadence, ScheduleInfo, TimeZoneName};

mod catalog;
mod change;
mod retry;
mod submit;
pub use change::FieldChange;
pub use submit::Received;
#[cfg(test)]
mod tests;

/// Identity of one open form/selected host, from the common nonreused issuer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visit(Ticket);
#[derive(Debug, Default)]
enum State {
    #[default]
    Closed,
    ChoosingHost {
        visit: Visit,
        draft: Box<Draft>,
    },
    Active(Box<Active>),
}
#[derive(Debug)]
struct Active {
    visit: Visit,
    connection: ConnectionContext,
    draft: Draft,
    project: Option<vessel_proto::ProjectId>,
    authority: Authority,
    targets: io::Mailbox,
    options: io::Mailbox,
    selected_catalog: Option<catalog::Catalog>,
    preview: io::Mailbox,
    saving: io::Mailbox,
}
impl Active {
    fn new(visit: Visit, connection: ConnectionContext, draft: Draft) -> Self {
        let project = draft
            .target()
            .and_then(|target| match target.spec().place.as_deref() {
                Some(vessel_proto::workspace::SessionPlace::Project { project, .. }) => {
                    Some(*project)
                }
                Some(vessel_proto::workspace::SessionPlace::Worktree { .. }) | None => None,
            });
        Self {
            visit,
            connection,
            draft,
            project,
            authority: Authority::default(),
            targets: io::Mailbox::default(),
            options: io::Mailbox::default(),
            selected_catalog: None,
            preview: io::Mailbox::default(),
            saving: io::Mailbox::default(),
        }
    }
    fn blocked(&self) -> bool {
        !self.authority.allows_actions()
            || self.saving.pending()
            || [&self.targets, &self.options, &self.preview, &self.saving]
                .into_iter()
                .filter_map(io::Mailbox::failure)
                .any(|failure| match failure {
                    Failure::ConnectionChanged | Failure::Unconfirmed => true,
                    // A read refusal is not evidence about Operate. Only a
                    // real write refusal is recorded in authority above.
                    Failure::Node(_)
                    | Failure::Older
                    | Failure::NotEnabled
                    | Failure::Offline
                    | Failure::TimedOut
                    | Failure::Busy
                    | Failure::Unexpected
                    | Failure::NoTransport
                    | Failure::Exhausted => false,
                })
    }
}
/// Pure form owner; closing never resets the ticket issuer or retries work.
#[derive(Debug, Default)]
pub struct Model {
    tickets: Tickets,
    state: State,
    /// Local identity exhaustion is explicit, without recycling a visit/ticket.
    pub failure: Option<Failure>,
}
/// Opening an admitted form requests only its target data, not a session/run.
#[derive(Debug)]
pub struct Opened {
    /// UI controls capture this particular form/host visit.
    pub visit: Visit,
    /// Node read through the existing schedules hub.
    pub request: io::Request,
}
impl Model {
    fn ticket(&mut self) -> Option<Ticket> {
        let ticket = self.tickets.issue();
        if ticket.is_none() {
            self.failure = Some(Failure::Exhausted);
        }
        ticket
    }
    /// Create initially needs a selected host; draft fields can already be shown.
    /// # Errors
    /// Unknown cadence is refused by the accepted draft boundary.
    pub fn create(
        &mut self,
        cadence: Cadence,
        viewer: TimeZoneName,
    ) -> Result<Option<Visit>, super::EditError> {
        let draft = Draft::create(cadence, viewer)?;
        if self.pending_save() {
            return Ok(None);
        }
        let Some(ticket) = self.ticket() else {
            return Ok(None);
        };
        let visit = Visit(ticket);
        self.state = State::ChoosingHost {
            visit,
            draft: Box::new(draft),
        };
        Ok(Some(visit))
    }
    /// Edit binds the opened definition and admission before any target read.
    /// # Errors
    /// An ended/unknown/unsupported definition cannot become an editable form.
    pub fn edit(
        &mut self,
        info: ScheduleInfo,
        connection: ConnectionContext,
        viewer: TimeZoneName,
    ) -> Result<Option<Opened>, super::EditError> {
        let draft = Draft::open(info, viewer)?;
        if self.pending_save() {
            return Ok(None);
        }
        Ok(self.install(connection, draft))
    }
    fn install(&mut self, connection: ConnectionContext, draft: Draft) -> Option<Opened> {
        let visit = Visit(self.ticket()?);
        let request = io::Request::on(connection.clone(), self.ticket()?, io::Operation::Targets);
        let mut active = Active::new(visit, connection, draft);
        active.targets.begin(request.clone());
        self.state = State::Active(Box::new(active));
        Some(Opened { visit, request })
    }
    /// A create can switch hosts; an edit's original machine is fixed.
    /// Re-selecting its current host is a no-op, preserving the text/caret visit.
    pub fn select_host(
        &mut self,
        expected: Visit,
        connection: ConnectionContext,
    ) -> Option<Opened> {
        if self.visit() != Some(expected)
            || self.blocked()
            || self.draft()?.opened().is_some()
            || self.connection() == Some(&connection)
        {
            return None;
        }
        // Allocate before taking the draft: exhaustion cannot discard user text.
        let visit = Visit(self.ticket()?);
        let request = io::Request::on(connection.clone(), self.ticket()?, io::Operation::Targets);
        let old = std::mem::take(&mut self.state);
        let mut draft = match old {
            State::ChoosingHost { draft, .. } => *draft,
            State::Active(active) => active.draft,
            State::Closed => return None,
        };
        if let Some(target) = draft.target_mut() {
            *target = None;
        }
        let mut active = Active::new(visit, connection, draft);
        active.targets.begin(request.clone());
        self.state = State::Active(Box::new(active));
        Some(Opened { visit, request })
    }
    /// Close only the exact visit, never an in-flight save. Uncertain work is
    /// not retried by close; the user must inspect the authoritative list.
    pub fn close(&mut self, expected: Visit) -> bool {
        if self.visit() != Some(expected) || self.pending_save() {
            return false;
        }
        self.state = State::Closed;
        true
    }
    /// Current visit, including an unselected-host create form.
    #[must_use]
    pub const fn visit(&self) -> Option<Visit> {
        match &self.state {
            State::Closed => None,
            State::ChoosingHost { visit, .. } => Some(*visit),
            State::Active(active) => Some(active.visit),
        }
    }
    /// Only the explicitly chosen admitted destination, never an alias guess.
    #[must_use]
    pub fn connection(&self) -> Option<&ConnectionContext> {
        match &self.state {
            State::Active(active) => Some(&active.connection),
            State::Closed | State::ChoosingHost { .. } => None,
        }
    }
    /// Original revision/full spec and current cached validation for drawing.
    #[must_use]
    pub fn draft(&self) -> Option<&Draft> {
        match &self.state {
            State::Closed => None,
            State::ChoosingHost { draft, .. } => Some(draft),
            State::Active(active) => Some(&active.draft),
        }
    }
    fn draft_mut(&mut self) -> Option<&mut Draft> {
        match &mut self.state {
            State::Closed => None,
            State::ChoosingHost { draft, .. } => Some(draft),
            State::Active(active) => Some(&mut active.draft),
        }
    }
    /// The four existing slots, without inventing a request payload just to
    /// inspect a spinner/error. Their stored requests retain the real method.
    pub fn operations(&self) -> impl Iterator<Item = &io::Mailbox> {
        let slots = match &self.state {
            State::Active(active) => Some([
                &active.targets,
                &active.options,
                &active.preview,
                &active.saving,
            ]),
            State::Closed | State::ChoosingHost { .. } => None,
        };
        slots.into_iter().flatten()
    }
    /// While saving no field/host change may replace the submitted definition.
    #[must_use]
    pub fn blocked(&self) -> bool {
        match &self.state {
            State::Active(active) => active.blocked(),
            State::Closed | State::ChoosingHost { .. } => false,
        }
    }
    /// Pending alone prevents dismissal; uncertainty may be dismissed explicitly.
    #[must_use]
    pub fn pending_save(&self) -> bool {
        match &self.state {
            State::Active(active) => active.saving.pending(),
            State::Closed | State::ChoosingHost { .. } => false,
        }
    }
}

impl Model {
    /// A fresh physical connection retires every slot, even before the Fleet
    /// differs. An in-flight save becomes uncertain and cannot be replayed.
    #[must_use]
    pub fn connected(&mut self, machine: &crate::schedules::load::MachineKey) -> bool {
        let State::Active(active) = &mut self.state else {
            return false;
        };
        let mut changed = false;
        for slot in [
            &mut active.targets,
            &mut active.options,
            &mut active.preview,
            &mut active.saving,
        ] {
            changed |= slot.connected(machine);
        }
        changed
    }
    /// Canonical Fleet retirement applies to the same slots as the reconnect
    /// notice. No replacement context is silently installed into an old visit.
    #[must_use]
    pub fn reconcile(&mut self, fleet: &crate::Fleet) -> bool {
        let State::Active(active) = &mut self.state else {
            return false;
        };
        let mut changed = false;
        for slot in [
            &mut active.targets,
            &mut active.options,
            &mut active.preview,
            &mut active.saving,
        ] {
            changed |= slot.reconcile(fleet);
        }
        changed
    }
}

impl Model {
    /// Apply only the same visit's existing page grant, without erasing a node
    /// refusal or treating a failed scope report as a read-only device.
    pub fn reported_authority(&mut self, expected: Visit, authority: &Authority) -> bool {
        let State::Active(active) = &mut self.state else {
            return false;
        };
        if active.visit != expected {
            return false;
        }
        let before = active.authority.clone();
        match authority {
            Authority::Unknown => {}
            Authority::Scopes(scopes) => active.authority.reported(Some(scopes)),
            Authority::OperationForbidden => {
                active.authority.refused(vessel_proto::ErrorCode::Forbidden);
            }
        }
        active.authority != before
    }
    /// The caller shows the existing bilingual read-only sentence once.
    #[must_use]
    pub fn authority(&self) -> Option<&Authority> {
        match &self.state {
            State::Active(active) => Some(&active.authority),
            State::Closed | State::ChoosingHost { .. } => None,
        }
    }
    /// Only this explicit switch changes archive policy; untouched specs remain
    /// exact when another field changes.
    pub fn finish(&mut self, expected: Visit, finish: super::definition::Finish) -> bool {
        if self.visit() != Some(expected) || self.blocked() {
            return false;
        }
        let Some(target) = self
            .draft_mut()
            .and_then(Draft::target_mut)
            .and_then(Option::as_mut)
        else {
            return false;
        };
        target.set_finish(finish);
        true
    }
}
