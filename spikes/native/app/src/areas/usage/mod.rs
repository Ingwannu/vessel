//! Settings Usage owns UI lifetime only; pure rules and source facts stay in view.
mod accounts;
mod actions;
mod answers;
mod clock;
mod sidebar;
pub(crate) use sidebar::sidebar_summary;
mod draw;
mod ledger;
mod modal;
mod options;
mod pins;
mod read_states;
mod rows;
mod session;
mod source_balances;
mod source_card;
mod source_controls;
mod source_policy;
mod source_reports;
mod source_tips;
mod source_windows;
mod windows;
pub(crate) use options::dialogs;
pub(crate) use session::{limit_notice, session_card};

use chrono::FixedOffset;
use vessel_app_view::usage::{
    Inventory, InventoryRefresh, LedgerSelection, Range, Reader, RequestFailure, SessionSelection,
};
use vessel_app_view::{
    connection::ConnectionContext,
    devices::{Target, Ticket, Tickets},
};
use vessel_proto::{
    limits::{ExternalLookup, UsageLimits},
    usage::UsageReport,
};

pub(crate) use draw::render;

/// Policy mutations have their own non-reused identity and are never retried silently.
#[derive(Debug)]
pub(super) struct PolicyPending {
    pub(super) ticket: Ticket,
    pub(super) machine: ConnectionContext,
    pub(super) policy: ExternalLookup,
}
#[derive(Debug)]
pub(super) struct SourcePending {
    pub(super) ticket: Ticket,
    pub(super) machine: ConnectionContext,
    pub(super) request: vessel_proto::usage_reports::SetUsageSourceLookup,
}

/// All requests live as long as their selected machine, never a visible label.
#[derive(Debug)]
pub(crate) struct State {
    pub(super) modal: vessel_app_view::usage::ModalCycle,
    pub(super) options: vessel_app_kit::popover::PopoverState,
    pub(super) options_open: Option<options::Opening>,
    pub(super) source_tips: source_tips::SourceTips,
    pub(super) options_anchor: vessel_app_kit::select::Anchor,
    pub(super) dialog: vessel_app_kit::DialogState,
    pub(super) host_list: vessel_app_kit::ComboboxState,
    pub(super) host_anchor: vessel_app_kit::select::Anchor,
    pub(super) target: Target,
    pub(super) connection: Option<ConnectionContext>,
    pub(super) range: Range,
    pub(super) session: Reader<SessionSelection, UsageLimits>,
    pub(super) session_owner: Option<vessel_proto::SessionId>,
    pub(super) session_epoch: Option<vessel_proto::EventSeq>,
    pub(super) ledger: Reader<LedgerSelection, UsageReport>,
    pub(super) summary: Reader<ConnectionContext, Inventory>,
    pub(super) summary_place: vessel_app_kit::select::Anchor,
    pub(super) inventory: Reader<ConnectionContext, Inventory>,
    pub(super) refresh_account: InventoryRefresh,
    pub(super) policy_pending: Option<PolicyPending>,
    pub(super) source_pending: Option<SourcePending>,
    pub(super) policy_error: Option<RequestFailure>,
    pub(super) tickets: Tickets,
    pub(super) offset: FixedOffset,
}
impl State {
    pub(crate) fn new(
        window: &mut vessel_app_kit::ui::Window,
        cx: &mut vessel_app_kit::ui::Context<'_, crate::shell::Shell>,
    ) -> Self {
        clock::start(cx);
        Self {
            modal: vessel_app_view::usage::ModalCycle::default(),
            options: vessel_app_kit::popover::PopoverState::new(cx),
            options_open: None,
            source_tips: source_tips::SourceTips::default(),
            options_anchor: vessel_app_kit::select::Anchor::default(),
            dialog: vessel_app_kit::DialogState::new(cx),
            host_list: vessel_app_kit::ComboboxState::new(window, cx),
            host_anchor: vessel_app_kit::select::Anchor::default(),
            target: Target::Hidden,
            connection: None,
            range: Range::default(),
            session: Reader::default(),
            session_owner: None,
            session_epoch: None,
            ledger: Reader::default(),
            summary: Reader::default(),
            summary_place: vessel_app_kit::select::Anchor::default(),
            inventory: Reader::default(),
            refresh_account: InventoryRefresh::Cached,
            policy_pending: None,
            source_pending: None,
            policy_error: None,
            tickets: Tickets::default(),
            offset: *chrono::Local::now().offset(),
        }
    }
}
