//! Only the node elects a recipient. A grant is followed by an exact projection
//! recheck; uncertainty or a lost reply never authorizes local retry/display.
use super::node::{problem, wire_key};
use super::transport::Client;
use vessel_app_view::inbox::{
    Problem,
    admission::{Admission, Pending},
};
use vessel_proto::inbox::{InboxEntry, InboxItem, InboxRead};
use vessel_proto::notifications::{NotificationClaim, NotificationSource};

pub(super) async fn claim(client: &Client, pending: &Pending) -> Result<Admission, Problem> {
    if !current(
        client
            .call(vessel_proto::inbox::GetInboxEntry {
                key: wire_key(pending.key),
                fetch: vessel_proto::inbox::InboxFetch::Summary,
            })
            .await
            .map_err(problem)?
            .as_ref(),
        pending,
    ) {
        return Ok(Admission::Suppressed);
    }
    let decision = client
        .call(vessel_proto::notifications::ClaimNotification {
            source: NotificationSource::Session {
                key: wire_key(pending.key),
                cursor: vessel_proto::inbox::InboxCursor::new(pending.position.get()),
            },
        })
        .await
        .map_err(problem)?;
    match decision {
        NotificationClaim::Admitted => {
            if current(
                client
                    .call(vessel_proto::inbox::GetInboxEntry {
                        key: wire_key(pending.key),
                        fetch: vessel_proto::inbox::InboxFetch::Summary,
                    })
                    .await
                    .map_err(problem)?
                    .as_ref(),
                pending,
            ) {
                Ok(Admission::Admitted)
            } else {
                Ok(Admission::Suppressed)
            }
        }
        NotificationClaim::NotRecipient
        | NotificationClaim::Suppressed
        | NotificationClaim::NoAttention
        | NotificationClaim::AlreadyClaimed
        | NotificationClaim::NoPresentClient
        | NotificationClaim::Unknown => Ok(Admission::Suppressed),
    }
}
fn current(entry: Option<&InboxEntry>, pending: &Pending) -> bool {
    entry.is_some_and(|entry| {
        entry.key == wire_key(pending.key)
            && entry.cursor.get() == pending.position.get()
            && entry.read == InboxRead::Unread
            && match entry.item {
                InboxItem::Permission { .. }
                | InboxItem::Question { .. }
                | InboxItem::InSession { .. }
                | InboxItem::Finished
                | InboxItem::Limit => true,
                InboxItem::Failed | InboxItem::Unknown => false,
            }
    })
}
