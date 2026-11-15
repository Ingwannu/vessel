//! Read-only Sync calls, with safe projection before the event boundary.
//!
//! [Decision Log]
//! - Purpose: inspect what the selected machine actually reports, while
//!   one unavailable read must not erase successful sections.
//! - Constraint: raw manifest transports and peer error text may contain
//!   values that do not belong on screen or in a log.
//! - Alternatives: a fail-fast bundle read, or carrying raw replies to UI.
//! - Choice: five bounded independent reads; consume the manifest into
//!   names/metadata here and convert failures to codes before sending.
//! - Why: the event type cannot contain a tool command, URL or value.
//! - Cost: this is a last-read view, not an atomic or live sync transaction.
//!   Refresh reads again; no mutating call is offered here.

use vessel_app_view::devices::{MachineKey, Ticket};
use vessel_app_view::sync::{Problem, Snapshot};

/// One correlated read; the result's type has no secret-value slots.
#[derive(Debug)]
pub struct Answer {
    /// Identity reserved by the page, retained through partial results.
    pub ticket: Ticket,
    /// Either a safe snapshot or a dispatch failure.
    pub result: Result<Box<Snapshot>, Problem>,
}

impl crate::Link {
    /// Queue a read, or immediately settle a full/closed queue.
    ///
    /// # Errors
    /// The connection queue cannot accept a request, or this is the demo browser.
    pub fn request_sync(&self, machine: MachineKey, ticket: Ticket) -> Result<(), Problem> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            enqueue(&self.commands, machine, ticket)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _request = (machine, ticket);
            Err(Problem::Refused(vessel_proto::ErrorCode::Unsupported))
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn enqueue(
    commands: &tokio::sync::mpsc::Sender<crate::Command>,
    machine: MachineKey,
    ticket: Ticket,
) -> Result<(), Problem> {
    crate::offer(commands, crate::Command::Sync { machine, ticket })
        .map_err(|unsent| rejected(unsent.why))
}

/// What the Sync page says of a read the link did not take: a full queue
/// as the node's `unavailable` (try again), a stopped link as offline (the
/// devices page's rule).
#[must_use]
pub const fn rejected(why: crate::Rejected) -> Problem {
    match why {
        crate::Rejected::Full => Problem::Refused(vessel_proto::ErrorCode::Unavailable),
        crate::Rejected::Closed => Problem::Offline,
    }
}

/// A fixed bound across repeated navigation and refreshes. Permits cover
/// reply sends too, so a full event queue cannot retain unlimited snapshots.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub(crate) struct Hub(std::sync::Arc<tokio::sync::Semaphore>);
#[cfg(not(target_arch = "wasm32"))]
impl Default for Hub {
    fn default() -> Self {
        Self(std::sync::Arc::new(tokio::sync::Semaphore::new(8)))
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl crate::State {
    pub(crate) async fn sync(&self, machine: &MachineKey, ticket: Ticket) {
        let client = match self.client(machine.as_str()) {
            Ok(client) => client.clone(),
            Err(_not_connected) => {
                self.emit(crate::Event::Sync(Answer {
                    ticket,
                    result: Err(Problem::Offline),
                }))
                .await;
                return;
            }
        };
        let Ok(permit) = std::sync::Arc::clone(&self.sync_calls.0).try_acquire_owned() else {
            self.emit(crate::Event::Sync(Answer {
                ticket,
                result: Err(Problem::Refused(vessel_proto::ErrorCode::Unavailable)),
            }))
            .await;
            return;
        };
        let events = self.events.clone();
        drop(tokio::spawn(async move {
            let snapshot = read(&client).await;
            let _closed = events
                .send(crate::Event::Sync(Answer {
                    ticket,
                    result: Ok(Box::new(snapshot)),
                }))
                .await;
            drop(permit);
        }));
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn read(client: &vessel_client::NodeClient) -> Snapshot {
    use vessel_app_view::sync::{Published, Reads};
    // Each future returns safe data. A completed raw manifest is projected
    // immediately, not left in join!'s storage while another call stalls.
    let published = async {
        bounded(client.fleet_manifest())
            .await
            .map(Published::project)
    };
    let (published, members, nodes, peers, secrets) = tokio::join!(
        published,
        bounded(client.fleet_members()),
        bounded(client.call(vessel_proto::ListNodes)),
        bounded(client.call(vessel_proto::ListPeers)),
        bounded(client.secret_list())
    );
    Snapshot::new(
        client.node().id,
        client.node().name.clone(),
        Reads {
            published,
            members,
            nodes,
            peers,
            secrets,
        },
    )
}

#[cfg(not(target_arch = "wasm32"))]
async fn bounded<T>(
    read: impl Future<Output = Result<T, vessel_client::CallError>>,
) -> Result<T, Problem> {
    match tokio::time::timeout(std::time::Duration::from_secs(15), read).await {
        Ok(reply) => reply.map_err(problem),
        Err(_timeout) => Err(Problem::Refused(vessel_proto::ErrorCode::Unavailable)),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn problem(error: vessel_client::CallError) -> Problem {
    match error {
        vessel_client::CallError::Disconnected => Problem::Offline,
        vessel_client::CallError::Remote(error) => Problem::Refused(error.code),
        vessel_client::CallError::NotInProtocol { .. }
        | vessel_client::CallError::UnassignedProtocol { .. } => {
            Problem::Refused(vessel_proto::ErrorCode::Unsupported)
        }
        vessel_client::CallError::UnexpectedReply(_) => Problem::InvalidReply,
        vessel_client::CallError::RequestIdsExhausted => Problem::Exhausted,
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    #[test]
    fn an_unassigned_call_is_unsupported() {
        assert_eq!(
            problem(vessel_client::CallError::UnassignedProtocol {
                method: vessel_proto::Method::NodeInfo,
            }),
            Problem::Refused(vessel_proto::ErrorCode::Unsupported)
        );
    }
    #[test]
    fn a_peer_cannot_echo_a_value_in_an_error_message_or_log() {
        let safe = problem(vessel_client::CallError::Remote(
            vessel_proto::WireError::new(
                vessel_proto::ErrorCode::Forbidden,
                "PRIVATE-VALUE-MARKER",
            ),
        ));
        assert_eq!(safe, Problem::Refused(vessel_proto::ErrorCode::Forbidden));
        // Redaction assertions at the log boundary, not UI wording tests.
        assert!(!safe.to_string().contains("PRIVATE-VALUE-MARKER"));
        assert!(!format!("{safe:?}").contains("PRIVATE-VALUE-MARKER"));
        assert_eq!(
            problem(vessel_client::CallError::Disconnected),
            Problem::Offline
        );
        assert_eq!(
            problem(vessel_client::CallError::RequestIdsExhausted),
            Problem::Exhausted
        );
    }
    #[test]
    fn full_and_closed_dispatch_queues_always_settle_the_page() {
        let (commands, inbox) = tokio::sync::mpsc::channel(1);
        let mut tickets = vessel_app_view::devices::Tickets::default();
        let machine = MachineKey::from_book("home");
        enqueue(&commands, machine.clone(), tickets.next_ticket().unwrap()).unwrap();
        assert_eq!(
            enqueue(&commands, machine.clone(), tickets.next_ticket().unwrap()),
            Err(Problem::Refused(vessel_proto::ErrorCode::Unavailable))
        );
        drop(inbox);
        assert_eq!(
            enqueue(&commands, machine, tickets.next_ticket().unwrap()),
            Err(Problem::Offline)
        );
    }
    #[test]
    fn stalled_reads_and_reply_sends_have_a_fixed_concurrency_limit() {
        let hub = Hub::default();
        let permits: Vec<_> = (0..8)
            .map(|_| std::sync::Arc::clone(&hub.0).try_acquire_owned().unwrap())
            .collect();
        assert!(std::sync::Arc::clone(&hub.0).try_acquire_owned().is_err());
        drop(permits);
        assert!(std::sync::Arc::clone(&hub.0).try_acquire_owned().is_ok());
    }
}
