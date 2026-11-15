//! A delete must join resume/archive admission and survive loss of its RPC waiter.
use super::*;
use std::time::Duration;

#[tokio::test]
async fn archive_delete_waits_for_publication_admission_and_retains_its_commit() {
    vessel_testkit::within(Duration::from_secs(30), async {
        let fixture = Fixture::new().await;
        let id = fixture
            .sessions
            .create(fixture.structured(), LaunchOptions::default(), None)
            .await
            .unwrap()
            .id;
        let Found::Live { handle, .. } = fixture.sessions.find(id).await.unwrap() else {
            panic!("missing created session")
        };
        let _ready = ready(&handle).await.unwrap();
        handle.kill();
        fixture.settle().await;
        assert!(matches!(
            fixture.sessions.find(id).await.unwrap(),
            Found::Stored(_)
        ));

        let admission = fixture.sessions.marking.lock().await;
        assert_eq!(fixture.tasks.len(), 0);
        let sessions = Arc::clone(&fixture.sessions);
        let mut request = tokio::spawn(async move { sessions.delete(id).await });
        // Observe actual node-owned work, rather than treating one Pending poll
        // as proof that deletion reached the publication barrier.
        tokio::select! {
            result = &mut request => panic!("delete escaped held admission: {result:?}"),
            () = async { while fixture.tasks.len() == 0 { tokio::task::yield_now().await; } } => {}
        }
        assert!(fixture.store.session(id).await.unwrap().is_some());
        request.abort();
        assert!(request.await.unwrap_err().is_cancelled());
        assert_eq!(fixture.tasks.len(), 1);
        assert!(fixture.store.session(id).await.unwrap().is_some());
        drop(admission);
        fixture.settle().await;
        assert!(fixture.store.session(id).await.unwrap().is_none());
        assert!(fixture.runtime.get(id).is_none());
        fixture.stop().await;
    })
    .await
    .expect("delete admission timed out");
}
