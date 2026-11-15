//! Read-only Models snapshots and refresh correlation, with no UI or I/O.
//!
//! [Decision Log]
//! - Purpose: show only facts read from the selected machine, preserving
//!   its last snapshot while a refresh runs or fails.
//! - Constraint: model/profile list methods arrived at different protocol
//!   versions; an unsupported method must not look like an empty list.
//! - Alternatives: guess agent capabilities; clear rows on every refresh;
//!   cache every visited machine. None is needed for read-only settings.
//! - Choice: one current page, reset on a machine change, ticketed answers
//!   and separately typed unsupported facts. Link tasks stay bounded.
//! - Cost: switching back reads another snapshot; no stale fact can claim
//!   to describe a different machine.
use vessel_proto::agents::AgentInfo;
use vessel_proto::{ModelRoute, ProfileInfo, RequestId};

/// A list the node actually sent, distinct from an unsupported operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Listed<T> {
    /// The method exists; empty is a real, successful empty result.
    Available(Vec<T>),
    /// The machine speaks a protocol older than this list method.
    OlderNode,
}
/// Facts read through one selected machine's connection.
#[derive(Debug)]
pub struct Snapshot {
    /// The machine's model/provider definitions, not compatibility guesses.
    pub routes: Listed<ModelRoute>,
    /// Every configured profile, including generated defaults.
    pub profiles: Listed<ProfileInfo>,
    /// Definitions used only to resolve added-agent display labels.
    pub agents: Vec<AgentInfo>,
}
/// The current machine's read-only page. The area resets it on a switch.
#[derive(Debug)]
pub struct Page<E> {
    snapshot: Option<Snapshot>,
    pending: Option<RequestId>,
    problem: Option<E>,
}
impl<E> Default for Page<E> {
    fn default() -> Self {
        Self {
            snapshot: None,
            pending: None,
            problem: None,
        }
    }
}
impl<E> Page<E> {
    /// Last successful node snapshot, including during a failed refresh.
    #[must_use]
    pub const fn snapshot(&self) -> Option<&Snapshot> {
        self.snapshot.as_ref()
    }
    /// Active request, echoed by the link's answer.
    #[must_use]
    pub const fn pending(&self) -> Option<RequestId> {
        self.pending
    }
    /// Typed last failure; its Display implementation is not translated UI.
    #[must_use]
    pub const fn problem(&self) -> Option<&E> {
        self.problem.as_ref()
    }
    /// Start one refresh, retaining a successful snapshot. A second ask is
    /// rejected without replacing the first ticket or dropping facts.
    fn start(&mut self, ticket: RequestId) -> bool {
        if self.pending.is_some() {
            return false;
        }
        self.pending = Some(ticket);
        self.problem = None;
        true
    }
    /// Apply only the current ticket. The machine key is also checked by
    /// the area before reaching this method; a reset has no active ticket.
    /// Returns false for a stale, duplicate or unsolicited answer.
    fn received(&mut self, ticket: RequestId, result: Result<Snapshot, E>) -> bool {
        if self.pending != Some(ticket) {
            return false;
        }
        self.pending = None;
        match result {
            Ok(snapshot) => {
                self.snapshot = Some(snapshot);
                self.problem = None;
            }
            Err(error) => self.problem = Some(error),
        }
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn empty() -> Snapshot {
        Snapshot {
            routes: Listed::Available(vec![]),
            profiles: Listed::Available(vec![]),
            agents: vec![],
        }
    }
    #[test]
    fn unsolicited_stale_and_duplicate_answers_cannot_replace_the_current_snapshot() {
        let mut page = Page::<&str>::default();
        let first = RequestId::new(1);
        let next = RequestId::new(2);
        assert!(!page.received(first, Ok(empty())));
        assert!(page.start(first));
        assert!(!page.start(next));
        assert_eq!(page.pending(), Some(first));
        assert!(!page.received(next, Err("stale")));
        assert!(page.problem().is_none());
        assert!(page.received(first, Ok(empty())));
        assert!(!page.received(first, Err("duplicate")));
        assert!(
            matches!(page.snapshot().unwrap().routes,Listed::Available(ref rows) if rows.is_empty())
        );
        assert!(page.problem().is_none());
    }
    #[test]
    fn a_failed_refresh_keeps_real_empty_lists_but_an_older_node_remains_distinct() {
        let mut page = Page::<&str>::default();
        let first = RequestId::new(1);
        let refresh = RequestId::new(2);
        assert!(page.start(first));
        assert!(page.received(first, Ok(empty())));
        assert!(page.start(refresh));
        assert!(page.snapshot().is_some());
        assert!(page.received(refresh, Err("disconnected")));
        assert_eq!(page.problem(), Some(&"disconnected"));
        assert!(
            matches!(page.snapshot().unwrap().routes,Listed::Available(ref rows) if rows.is_empty())
        );
        let retry = RequestId::new(3);
        assert!(page.start(retry));
        assert!(page.received(
            retry,
            Ok(Snapshot {
                routes: Listed::OlderNode,
                profiles: Listed::OlderNode,
                agents: vec![]
            })
        ));
        assert!(matches!(page.snapshot().unwrap().routes, Listed::OlderNode));
        assert!(matches!(
            page.snapshot().unwrap().profiles,
            Listed::OlderNode
        ));
        assert!(page.problem().is_none());
        assert_eq!(page.pending(), None);
    }
    #[test]
    fn resetting_for_a_new_machine_rejects_the_previous_machine_ticket() {
        let old = RequestId::new(1);
        let mut page = Page::<&str>::default();
        assert!(page.start(old));
        page = Page::default();
        assert!(!page.received(old, Ok(empty())));
        assert!(page.snapshot().is_none());
        assert!(page.start(RequestId::new(2)));
        assert!(!page.received(old, Err("old machine")));
        assert!(page.problem().is_none());
    }
}

/// A page's machine identity and a ticket sequence that survives navigation.
/// Keeping the sequence outside Page prevents A → B → A from reusing A's
/// first ticket while its original network answer is still in flight.
#[derive(Debug)]
pub struct State<E> {
    machine: Option<crate::agent_status::pages::MachineKey>,
    next: u64,
    page: Page<E>,
}
impl<E> Default for State<E> {
    fn default() -> Self {
        Self {
            machine: None,
            next: 0,
            page: Page::default(),
        }
    }
}
/// Why a page read cannot be queued.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum StartProblem {
    /// There is no selected machine.
    #[error("no machine selected")]
    NoMachine,
    /// The current machine's read is still pending.
    #[error("model read already pending")]
    Busy,
    /// Tickets cannot wrap and alias a delayed answer.
    #[error("model request identities exhausted")]
    Exhausted,
}
impl<E> State<E> {
    /// Select a validated book key. An actual change clears only the page,
    /// never the request sequence. Reselecting the same key keeps its facts.
    pub fn select(&mut self, machine: Option<crate::agent_status::pages::MachineKey>) {
        if machine != self.machine {
            self.machine = machine;
            self.page = Page::default();
        }
    }
    /// The current machine, used to tag the outgoing read.
    #[must_use]
    pub const fn machine(&self) -> Option<&crate::agent_status::pages::MachineKey> {
        self.machine.as_ref()
    }
    /// Facts/pending/error for the selected machine only.
    #[must_use]
    pub const fn page(&self) -> &Page<E> {
        &self.page
    }
    /// Allocate a fresh identity across all machine switches.
    ///
    /// # Errors
    /// No machine, an existing read or exhausted identity leaves state intact.
    pub fn begin(&mut self) -> Result<RequestId, StartProblem> {
        if self.machine.is_none() {
            return Err(StartProblem::NoMachine);
        }
        if self.page.pending().is_some() {
            return Err(StartProblem::Busy);
        }
        let next = self.next.checked_add(1).ok_or(StartProblem::Exhausted)?;
        let ticket = RequestId::new(self.next);
        self.next = next;
        let _started = self.page.start(ticket);
        Ok(ticket)
    }
    /// Both machine and never-reused ticket must match the active read.
    pub fn received(
        &mut self,
        machine: &crate::agent_status::pages::MachineKey,
        ticket: RequestId,
        result: Result<Snapshot, E>,
    ) -> bool {
        if self.machine.as_ref() != Some(machine) {
            return false;
        }
        self.page.received(ticket, result)
    }
}
#[cfg(test)]
mod navigation_tests {
    use super::*;
    use crate::agent_status::pages::MachineKey;
    #[test]
    fn same_machine_selection_keeps_pending_facts_and_exhaustion_is_nonmutating() {
        let machine = MachineKey::from_book("one");
        let mut state = State::<&str>::default();
        state.select(Some(machine.clone()));
        let ticket = state.begin().unwrap();
        state.select(Some(machine.clone()));
        assert_eq!(state.page().pending(), Some(ticket));
        assert!(state.received(&machine, ticket, Err("retained failure")));
        state.select(Some(machine.clone()));
        assert_eq!(state.page().problem(), Some(&"retained failure"));
        state.next = u64::MAX;
        assert_eq!(state.begin(), Err(StartProblem::Exhausted));
        assert_eq!(state.next, u64::MAX);
        assert_eq!(state.page().pending(), None);
        assert_eq!(state.page().problem(), Some(&"retained failure"));
        state.select(None);
        assert_eq!(state.machine(), None);
        assert_eq!(state.page().problem(), None);
        assert_eq!(state.next, u64::MAX);
    }

    #[test]
    fn returning_to_a_machine_does_not_reuse_its_old_ticket() {
        let a = MachineKey::from_book("one");
        let b = MachineKey::from_book("two");
        let mut state = State::<&str>::default();
        assert_eq!(state.begin(), Err(StartProblem::NoMachine));
        state.select(Some(a.clone()));
        let old = state.begin().unwrap();
        assert_eq!(state.begin(), Err(StartProblem::Busy));
        state.select(Some(b.clone()));
        let other = state.begin().unwrap();
        state.select(Some(a.clone()));
        let current = state.begin().unwrap();
        assert_ne!(old, current);
        assert!(!state.received(&a, old, Err("delayed A")));
        assert!(!state.received(&b, other, Err("delayed B")));
        assert_eq!(state.page().pending(), Some(current));
        assert_eq!(state.page().problem(), None);
        assert!(state.received(&a, current, Err("current A")));
        assert_eq!(state.page().problem(), Some(&"current A"));
    }
}
