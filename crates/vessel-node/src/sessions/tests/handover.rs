//! A synchronous receiver wake pins the end notification's visibility boundary.
use super::Fixture;
use crate::persist::{self, Persisted};
use crate::sessions::{Found, Sessions};
use std::sync::{Arc, Mutex};
use std::task::{Context, Wake, Waker};
use std::time::Duration;
use vessel_proto::{EventSeq, SessionId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Presence {
    Present,
    Absent,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Visible {
    runtime: Presence,
    durability: Presence,
}
struct AtWake {
    sessions: Arc<Sessions>,
    session: SessionId,
    visible: Mutex<Option<Visible>>,
}
impl Wake for AtWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        // broadcast::send wakes registered receivers inline. Observe the
        // actual state before hand_over can execute its next statement;
        // this does not depend on another thread winning a short race.
        let runtime = match self.sessions.runtime.get(self.session) {
            Some(_) => Presence::Present,
            None => Presence::Absent,
        };
        let durability = if self
            .sessions
            .durability
            .read()
            .unwrap()
            .contains_key(&self.session)
        {
            Presence::Present
        } else {
            Presence::Absent
        };
        *self.visible.lock().unwrap() = Some(Visible {
            runtime,
            durability,
        });
    }
}

// Keep actual persistence setup separate from the notification-order assertion.
async fn persisted_session(fixture: &Fixture) -> (vessel_session::SessionHandle, Persisted) {
    let created = fixture.runtime.create(fixture.structured()).await.unwrap();
    let handle = created.handle;
    fixture.store.create_session(handle.info()).await.unwrap();
    let durability = fixture.sessions.track(&handle, EventSeq::ZERO);
    let _running = super::ready(&handle).await.unwrap();
    handle.kill();
    let persisted = persist::run(
        fixture.store.clone(),
        handle.clone(),
        created.durable,
        durability,
    )
    .await;
    assert_eq!(persisted, Persisted::Complete);
    (handle, persisted)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_end_notification_follows_runtime_and_durability_removal() {
    vessel_testkit::within(Duration::from_secs(60), async {
        let fixture = Fixture::new().await;
        let (handle, persisted) = persisted_session(&fixture).await;
        let id = handle.id();
        assert!(matches!(
            fixture.sessions.find(id).await.unwrap(),
            Found::Live { .. }
        ));
        let mut ended = fixture.sessions.ends();
        let observer = Arc::new(AtWake {
            sessions: Arc::clone(&fixture.sessions),
            session: id,
            visible: Mutex::new(None),
        });
        let waker = Waker::from(Arc::clone(&observer));
        let mut notification = Box::pin(ended.recv());
        assert!(
            notification
                .as_mut()
                .poll(&mut Context::from_waker(&waker))
                .is_pending()
        );
        fixture.sessions.hand_over(handle, persisted).await;
        assert_eq!(
            *observer.visible.lock().unwrap(),
            Some(Visible {
                runtime: Presence::Absent,
                durability: Presence::Absent
            })
        );
        assert_eq!(notification.await.unwrap(), id);
        assert!(matches!(
            fixture.sessions.find(id).await.unwrap(),
            Found::Stored(_)
        ));
        fixture.sessions.wait_stored(id).await.unwrap();
        fixture.stop().await;
    })
    .await
    .unwrap();
}
