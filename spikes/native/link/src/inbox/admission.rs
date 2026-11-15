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
                | InboxItem::Failed
                | InboxItem::Limit => true,
                InboxItem::Unknown => false,
            }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_local_attention_still_requires_exact_unread_current_source() {
        let mut epochs = vessel_app_view::connection::ConnectionEpochs::default();
        let source = vessel_app_view::inbox::Source::new(
            epochs
                .admit(
                    vessel_app_view::connection::MachineKey::from_book("fixture"),
                    vessel_proto::NodeId::random(),
                )
                .unwrap(),
        )
        .unwrap();
        let mut entry = InboxEntry {
            cursor: vessel_proto::inbox::InboxCursor::new(1),
            key: vessel_proto::inbox::InboxKey {
                session: vessel_proto::SessionId::random(),
                seq: vessel_proto::EventSeq::ZERO,
            },
            title: None,
            project: None,
            project_name: None,
            at: chrono::DateTime::UNIX_EPOCH,
            read: InboxRead::Unread,
            item: InboxItem::Failed,
        };
        let pending = Pending {
            key: vessel_app_view::inbox::Key {
                node: source.node(),
                session: entry.key.session,
                seq: entry.key.seq,
            },
            source,
            position: vessel_app_view::inbox::Position::new(entry.cursor.get()),
            request: vessel_app_view::inbox::Inbox::default().ticket().unwrap(),
        };
        assert!(current(Some(&entry), &pending));
        entry.read = InboxRead::Read;
        assert!(!current(Some(&entry), &pending));
        entry.read = InboxRead::Unread;
        entry.cursor = vessel_proto::inbox::InboxCursor::new(2);
        assert!(!current(Some(&entry), &pending));
        entry.cursor = vessel_proto::inbox::InboxCursor::new(pending.position.get());
        entry.item = InboxItem::Unknown;
        assert!(!current(Some(&entry), &pending));
    }
}
