//! Fixed admission covers both node awaits and delivery into the bounded UI queue.
use super::{Answer, Ask};
use std::{sync::Arc, time::Duration};
use tokio::sync::{Semaphore, mpsc};
use vessel_app_view::usage::{
    AccountReport, Inventory, InventoryRefresh, RequestFailure, account_binding,
    configured_accounts,
};
use vessel_client::{CallError, NodeClient};
use vessel_proto::{
    ErrorCode, ListProfiles,
    limits::{GetUsageLimits, LimitRefresh, LimitsScope, SetUsageLookup},
    usage::{QueryUsage, UsagePeriod, UsageScope},
};

const ADMITTED_CALLS: usize = 4;
const LEDGER_DEADLINE: Duration = Duration::from_secs(30);
const INVENTORY_DEADLINE: Duration = Duration::from_secs(60);
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
        let client = match self.client(ask.machine().as_str()) {
            Ok(client) => client.clone(),
            Err(_offline) => {
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
        Ask::Lookup { .. } => POLICY_DEADLINE,
        Ask::Session { .. } => SESSION_DEADLINE,
    }
}

async fn execute(client: &NodeClient, ask: Ask) -> Answer {
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

async fn inventory(
    client: &NodeClient,
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
    Ok(Inventory {
        accounts: reports,
        refresh_problem: (!matched).then_some(RequestFailure::Refused(ErrorCode::NotFound)),
    })
}

fn problem(error: CallError) -> RequestFailure {
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
