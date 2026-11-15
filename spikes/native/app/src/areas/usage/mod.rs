//! Settings Usage owns UI lifetime only; pure rules and source facts stay in view.
mod accounts;
mod actions;
mod draw;
mod ledger;
mod rows;
mod session;
mod windows;
pub(crate) use session::{limit_notice, session_card};

use chrono::FixedOffset;
use vessel_app_view::devices::{MachineKey, Target, Ticket, Tickets};
use vessel_app_view::usage::{
    DisplayAs, Inventory, LedgerSelection, Range, Reader, RequestFailure, SessionSelection,
};
use vessel_proto::{
    limits::{ExternalLookup, LimitAccount, UsageLimits},
    usage::UsageReport,
};

pub(crate) use draw::render;

/// Policy mutations have their own non-reused identity and are never retried silently.
#[derive(Debug)]
pub(super) struct PolicyPending {
    pub(super) ticket: Ticket,
    pub(super) machine: MachineKey,
    pub(super) policy: ExternalLookup,
}

/// All requests live as long as their selected machine, never a visible label.
#[derive(Debug)]
pub(crate) struct State {
    pub(super) target: Target,
    pub(super) range: Range,
    pub(super) display: DisplayAs,
    pub(super) session: Reader<SessionSelection, UsageLimits>,
    pub(super) session_owner: Option<vessel_proto::SessionId>,
    pub(super) session_epoch: Option<vessel_proto::EventSeq>,
    pub(super) ledger: Reader<LedgerSelection, UsageReport>,
    pub(super) inventory: Reader<MachineKey, Inventory>,
    pub(super) refresh_account: Option<LimitAccount>,
    pub(super) policy_pending: Option<PolicyPending>,
    pub(super) policy_error: Option<RequestFailure>,
    pub(super) tickets: Tickets,
    pub(super) offset: FixedOffset,
}
impl State {
    pub(crate) fn new() -> Self {
        Self {
            target: Target::Hidden,
            range: Range::default(),
            display: DisplayAs::default(),
            session: Reader::default(),
            session_owner: None,
            session_epoch: None,
            ledger: Reader::default(),
            inventory: Reader::default(),
            refresh_account: None,
            policy_pending: None,
            policy_error: None,
            tickets: Tickets::default(),
            offset: *chrono::Local::now().offset(),
        }
    }
}
