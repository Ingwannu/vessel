//! A read begun before a mutation result must not resurrect its older fleet rows.
//! The watch stores one revision; overflow disables publication until link restart.
use crate::{ActKind, Event};
use tokio::sync::{mpsc, watch};
use vessel_app_view::Fleet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Revision(u64);
impl Revision {
    fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}
#[derive(Debug, Clone)]
pub(crate) struct Projection(watch::Sender<Option<Revision>>);
impl Default for Projection {
    fn default() -> Self {
        Self(watch::channel(Some(Revision(0))).0)
    }
}
impl Projection {
    pub(crate) fn capture(&self) -> Option<Revision> {
        *self.0.borrow()
    }
    #[cfg(test)]
    fn accepts(&self, read: Option<Revision>) -> bool {
        read.is_some() && read == *self.0.borrow()
    }
    pub(crate) fn changed(&self) {
        self.0
            .send_modify(|revision| *revision = revision.and_then(Revision::next));
    }
    pub(crate) async fn publish(
        &self,
        read: Option<Revision>,
        events: &mpsc::Sender<Event>,
        fleet: Fleet,
    ) {
        let Ok(permit) = events.reserve().await else {
            return;
        };
        // Reserving bounded channel capacity may yield. Recheck and enqueue
        // under the revision's write lock, with no await between them, so a
        // mutation can never enqueue its acknowledgement ahead of an old fleet.
        let _unchanged = self.0.send_if_modified(|current| {
            if read.is_some() && *current == read {
                permit.send(Event::Fleet(fleet));
            }
            false
        });
    }
    pub(crate) fn after(&self, kind: ActKind) {
        match kind {
            ActKind::Info | ActKind::Conversation | ActKind::Log | ActKind::Terminal => {}
            ActKind::Rename
            | ActKind::Mark
            | ActKind::StartAgain
            | ActKind::Delete
            | ActKind::Kill
            | ActKind::MoveProfile
            | ActKind::Queue
            | ActKind::Answer
            | ActKind::Permit
            | ActKind::SetOption
            | ActKind::Wake
            | ActKind::Rewind
            | ActKind::Fork => self.changed(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_inflight_snapshot_cannot_resurrect_a_committed_archive_or_delete() {
        let projection = Projection::default();
        let old_read = projection.capture();
        projection.after(ActKind::Mark);
        assert!(!projection.accepts(old_read));
        let newer = projection.capture();
        assert!(projection.accepts(newer));
        projection.after(ActKind::Delete);
        assert!(!projection.accepts(newer));
    }
    #[test]
    fn passive_reads_do_not_starve_periodic_fleet_refresh() {
        let projection = Projection::default();
        let read = projection.capture();
        for kind in [ActKind::Info, ActKind::Conversation, ActKind::Log] {
            projection.after(kind);
        }
        assert!(projection.accepts(read));
    }
    #[test]
    fn exhausted_revision_stays_unpublishable_instead_of_recycling_identity() {
        let projection = Projection(watch::channel(Some(Revision(u64::MAX))).0);
        let last = projection.capture();
        projection.changed();
        assert!(!projection.accepts(last));
        assert!(!projection.accepts(projection.capture()));
        projection.changed();
        assert!(projection.capture().is_none());
    }

    #[tokio::test]
    async fn publication_rechecks_after_waiting_for_bounded_channel_capacity() {
        let projection = Projection::default();
        let read = projection.capture();
        let (events, mut receiving) = mpsc::channel(1);
        events
            .send(Event::Fleet(Fleet {
                machines: Vec::new(),
                read_at: None,
            }))
            .await
            .unwrap();
        let publish = projection.publish(
            read,
            &events,
            Fleet {
                machines: Vec::new(),
                read_at: None,
            },
        );
        tokio::pin!(publish);
        std::future::poll_fn(|cx| {
            assert!(publish.as_mut().poll(cx).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        projection.after(ActKind::Mark);
        assert!(matches!(receiving.recv().await, Some(Event::Fleet(_))));
        publish.await;
        assert!(
            receiving.try_recv().is_err(),
            "the old fleet must not follow the mutation acknowledgement"
        );
        projection
            .publish(
                projection.capture(),
                &events,
                Fleet {
                    machines: Vec::new(),
                    read_at: None,
                },
            )
            .await;
        assert!(matches!(receiving.recv().await, Some(Event::Fleet(_))));
    }
}
