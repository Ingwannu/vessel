//! Fixed admission covers both node awaits and delivery into the bounded UI queue.
use super::{Answer, Ask};
use crate::connections::{RequestFailure as ConnectionFailure, SelectedConnection};
use std::{sync::Arc, time::Duration};
use tokio::sync::{Semaphore, mpsc};
use vessel_app_view::connection::Refusal;
use vessel_app_view::usage::{
    AccountReport, Inventory, InventoryRefresh, RequestFailure, account_binding,
    configured_accounts,
};
use vessel_client::CallError;
use vessel_proto::{
    ErrorCode, ListProfiles,
    limits::{GetUsageLimits, LimitRefresh, LimitsScope, SetUsageLookup},
    usage::{QueryUsage, UsagePeriod, UsageScope},
};

const ADMITTED_CALLS: usize = 4;
const LEDGER_DEADLINE: Duration = Duration::from_secs(30);
const INVENTORY_DEADLINE: Duration = Duration::from_secs(120);
const POLICY_DEADLINE: Duration = Duration::from_secs(15);
const SESSION_DEADLINE: Duration = Duration::from_secs(30);

/// A reconnect or repeated navigation cannot create an unbounded task population.
#[derive(Debug)]
pub(crate) struct Hub(Arc<Semaphore>);
impl Default for Hub {
    fn default() -> Self {
        Self(Arc::new(Semaphore::new(ADMITTED_CALLS)))
    }
}
impl crate::State {
    pub(crate) async fn usage(&self, ask: Ask) {
        let client = match self.selected_connection(ask.connection()) {
            Ok(client) => client,
            Err(_changed) => {
                self.emit(crate::Event::Usage(ask.refused(RequestFailure::Offline)))
                    .await;
                return;
            }
        };
        let Ok(permit) = Arc::clone(&self.usage_calls.0).try_acquire_owned() else {
            self.emit(crate::Event::Usage(ask.refused(RequestFailure::Busy)))
                .await;
            return;
        };
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let failure = ask.clone();
            let answer = tokio::time::timeout(deadline(&ask), execute(&client, ask))
                .await
                .unwrap_or_else(|_elapsed| failure.refused(RequestFailure::TimedOut));
            send(&events, answer).await;
            // A full UI queue cannot retain unlimited completed snapshots outside admission.
            drop(permit);
        }));
    }
}
async fn send(events: &mpsc::Sender<crate::Event>, answer: Answer) {
    let _closed = events.send(crate::Event::Usage(answer)).await;
}
fn deadline(ask: &Ask) -> Duration {
    match ask {
        Ask::Accounts { .. } => INVENTORY_DEADLINE,
        Ask::Ledger(_) => LEDGER_DEADLINE,
        Ask::Lookup { .. } | Ask::SourceLookup { .. } => POLICY_DEADLINE,
        Ask::Session { .. } => SESSION_DEADLINE,
    }
}

async fn execute(client: &SelectedConnection, ask: Ask) -> Answer {
    match ask {
        Ask::Accounts {
            surface,
            request,
            refresh,
        } => {
            let result = inventory(client, &refresh).await;
            Answer::Accounts {
                surface,
                request,
                result,
            }
        }
        Ask::Ledger(request) => {
            let period = match request.selection.range {
                vessel_app_view::usage::Range::Day => UsagePeriod::Day,
                vessel_app_view::usage::Range::Week => UsagePeriod::Week,
            };
            let result = client
                .call(QueryUsage {
                    scope: UsageScope::Machine,
                    period,
                    after: None,
                })
                .await
                .map_err(problem);
            Answer::Ledger { request, result }
        }
        Ask::Lookup {
            machine,
            ticket,
            policy,
        } => {
            let result = client
                .call(SetUsageLookup { policy })
                .await
                .map_err(problem);
            Answer::Lookup {
                machine,
                ticket,
                result,
            }
        }
        Ask::SourceLookup {
            machine,
            ticket,
            request,
        } => Answer::SourceLookup {
            machine,
            ticket,
            result: client.call(request).await.map_err(problem),
        },
        Ask::Session { request, refresh } => {
            let result = client
                .call(GetUsageLimits {
                    scope: LimitsScope::Session {
                        session: request.selection.session,
                    },
                    refresh,
                })
                .await
                .map_err(problem);
            Answer::Session { request, result }
        }
    }
}

async fn saved_inventory(
    client: &SelectedConnection,
    refresh: &InventoryRefresh,
) -> Result<Inventory, RequestFailure> {
    let profiles = client
        .call(ListProfiles { harness: None })
        .await
        .map_err(problem)?;
    let accounts =
        configured_accounts(&profiles).map_err(|_invalid| RequestFailure::InvalidReply)?;
    let mut matched = !matches!(refresh, InventoryRefresh::Account(_));
    let mut reports = Vec::with_capacity(accounts.len());
    for account in accounts {
        let binding = account_binding(&account);
        let refresh = if refresh.includes(&binding) {
            matched = true;
            LimitRefresh::Refresh
        } else {
            LimitRefresh::Cached
        };
        let result = client
            .call(GetUsageLimits {
                scope: LimitsScope::Account {
                    account: binding.clone(),
                },
                refresh,
            })
            .await
            .map_err(problem)
            .and_then(|limits| {
                if limits.account == binding {
                    Ok(limits)
                } else {
                    Err(RequestFailure::InvalidReply)
                }
            });
        reports.push(AccountReport { account, result });
    }
    Ok(Inventory::Saved {
        accounts: reports,
        refresh_problem: if matches!(refresh, InventoryRefresh::Report(_)) {
            Some(RequestFailure::Unsupported)
        } else {
            (!matched).then_some(RequestFailure::Refused(ErrorCode::NotFound))
        },
    })
}
async fn inventory(
    client: &SelectedConnection,
    refresh: &InventoryRefresh,
) -> Result<Inventory, RequestFailure> {
    if vessel_proto::Method::ListUsageReports.since().is_none() {
        return saved_inventory(client, refresh).await;
    }
    match source_inventory(client, refresh).await {
        Err(RequestFailure::Unsupported) => saved_inventory(client, refresh).await,
        answer => answer,
    }
}
async fn source_inventory(
    client: &SelectedConnection,
    refresh: &InventoryRefresh,
) -> Result<Inventory, RequestFailure> {
    use vessel_proto::usage_reports::{
        InspectUsageReport, ListUsageReports, UsageReportLookup, UsageReportScope,
    };
    let refresh_problem = match refresh {
        InventoryRefresh::Report(id) => match client
            .call(InspectUsageReport {
                id: id.clone(),
                refresh: LimitRefresh::Refresh,
            })
            .await
        {
            Ok(UsageReportLookup::Found { report, .. }) if report.id() == id => None,
            Ok(UsageReportLookup::Found { .. }) => return Err(RequestFailure::InvalidReply),
            Ok(UsageReportLookup::NotFound { .. }) => {
                Some(RequestFailure::Refused(ErrorCode::NotFound))
            }
            Ok(UsageReportLookup::Off) => Some(RequestFailure::Refused(ErrorCode::Conflict)),
            Err(error) => Some(problem(error)),
        },
        InventoryRefresh::Account(_) => Some(RequestFailure::Unsupported),
        InventoryRefresh::Cached | InventoryRefresh::All => None,
    };
    let reports = client
        .call(ListUsageReports {
            scope: UsageReportScope::Machine,
            refresh: if *refresh == InventoryRefresh::All {
                LimitRefresh::Refresh
            } else {
                LimitRefresh::Cached
            },
        })
        .await
        .map_err(problem)?;
    let _encoded = reports
        .validate_budget()
        .map_err(|_budget| RequestFailure::InvalidReply)?;
    Ok(Inventory::Sources {
        reports,
        refresh_problem,
    })
}

fn problem(error: ConnectionFailure) -> RequestFailure {
    match error {
        ConnectionFailure::Refused(Refusal::Changed | Refusal::Closed) => RequestFailure::Offline,
        ConnectionFailure::Call(error) => call_problem(error),
    }
}
fn call_problem(error: CallError) -> RequestFailure {
    match error {
        CallError::Disconnected => RequestFailure::Offline,
        CallError::Remote(error) => RequestFailure::Refused(error.code),
        CallError::NotInProtocol { .. } | CallError::UnassignedProtocol { .. } => {
            RequestFailure::Unsupported
        }
        CallError::UnexpectedReply(_) | CallError::RequestIdsExhausted => {
            RequestFailure::InvalidReply
        }
    }
}

#[cfg(test)]
mod tests;
