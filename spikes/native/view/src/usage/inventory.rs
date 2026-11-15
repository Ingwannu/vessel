//! A configured binding and its independent account-limit result stay together.
use super::{Account, RequestFailure};
use vessel_proto::limits::{ExternalLookup, LimitAccount, UsageLimits};

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
pub struct Inventory {
    /// The node's configured profile order, including implicit defaults.
    pub accounts: Vec<AccountReport>,
    /// An explicit refresh can lose its target during a configuration change.
    pub refresh_problem: Option<RequestFailure>,
}
impl Inventory {
    /// A concurrent policy change cannot silently become one authoritative value.
    #[must_use]
    pub fn policy(&self) -> Option<ExternalLookup> {
        let mut reported = self
            .accounts
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
