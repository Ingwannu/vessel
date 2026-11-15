//! A configured binding and its independent account-limit result stay together.
use super::{Account, RequestFailure};
use vessel_proto::limits::{ExternalLookup, LimitAccount, UsageLimits};
/// An explicit user refresh cannot be confused with a passive cached inventory read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum InventoryRefresh {
    /// The default has no credential or vendor I/O, including cache misses.
    #[default]
    Cached,
    /// Refresh exactly one configured account, leaving all others cached.
    Account(LimitAccount),
    /// Exact report discovered by the generic source registry; never parsed or replaced by a label.
    Report(vessel_proto::usage_reports::UsageReportId),
    /// Refresh every account in the selected machine's bounded current inventory.
    All,
}
impl InventoryRefresh {
    /// Compare canonical bindings; labels and currently selected defaults never match accounts.
    #[must_use]
    pub fn includes(&self, account: &LimitAccount) -> bool {
        match self {
            Self::Cached | Self::Report(_) => false,
            Self::Account(chosen) => chosen == account,
            Self::All => true,
        }
    }
}

/// A failed account never hides the accounts that answered successfully.
#[derive(Debug, Clone)]
pub struct AccountReport {
    /// Stable configured identity, separate from its human label.
    pub account: Account,
    /// Safe current outcome for this exact configured binding.
    pub result: Result<UsageLimits, RequestFailure>,
}

/// The node's profile inventory plus independently readable quota snapshots.
#[derive(Debug, Clone)]
pub enum Inventory {
    /// Older peers expose configured saved-login projections only.
    Saved {
        /// The node's configured profile order, including implicit defaults.
        accounts: Vec<AccountReport>,
        /// An explicit refresh can lose its target during configuration changes.
        refresh_problem: Option<RequestFailure>,
    },
    /// The authoritative bounded generic report/source model from a capable peer.
    Sources {
        /// Typed account identities, original observations and explicit source consent.
        reports: vessel_proto::usage_reports::UsageReports,
        /// Inspect can lose an account while refreshing its exact opaque id.
        refresh_problem: Option<RequestFailure>,
    },
}
impl Inventory {
    /// Saved projections are never mixed with a newer peer's source/account inventory.
    #[must_use]
    pub fn accounts(&self) -> &[AccountReport] {
        match self {
            Self::Saved { accounts, .. } => accounts,
            Self::Sources { .. } => &[],
        }
    }
    /// Generic reports remain in their canonical protocol model; there is no UI-side parser.
    #[must_use]
    pub const fn source_reports(&self) -> Option<&vessel_proto::usage_reports::UsageReports> {
        match self {
            Self::Sources { reports, .. } => Some(reports),
            Self::Saved { .. } => None,
        }
    }
    /// A mutation result does not silently become success merely because the cached read worked.
    #[must_use]
    pub const fn refresh_problem(&self) -> Option<RequestFailure> {
        match self {
            Self::Saved {
                refresh_problem, ..
            }
            | Self::Sources {
                refresh_problem, ..
            } => *refresh_problem,
        }
    }
    /// A concurrent policy change cannot silently become one authoritative value.
    #[must_use]
    pub fn policy(&self) -> Option<ExternalLookup> {
        if let Self::Sources { reports, .. } = self {
            return Some(reports.policy);
        }
        let mut reported = self
            .accounts()
            .iter()
            .filter_map(|account| account.result.as_ref().ok().map(|limits| limits.policy));
        let first = reported.next()?;
        reported.all(|policy| policy == first).then_some(first)
    }
}

/// Inventory uses an explicit profile, never an inferred current/default login.
#[must_use]
pub fn account_binding(account: &Account) -> LimitAccount {
    LimitAccount {
        harness: account.harness().clone(),
        profile: Some(account.profile().clone()),
    }
}

/// Independent UI lifetimes cannot consume one another's equal-valued tickets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventorySurface {
    /// Settings and the shared Usage modal.
    Accounts,
    /// The optional pinned summary, whose host may differ from Settings.
    Summary,
}
