//! Cancellation must discard results without releasing physically occupied capacity.

use super::*;
use crate::models::{PER_NODE, PER_SESSION, blocking_tests::Stage};
use crate::settings::Settings;
use tokio_util::task::TaskTracker;
use vessel_store::{DataDir, Store, StoreConfig};

/// Each scenario admits one real read before holding the rest of the existing pool.
const HELD_READS: usize = 1;

#[tokio::test]
async fn held_real_style_reads_keep_admission_and_node_drain_after_cancellation() {
    for cancel in [Cancel::Typed, Cancel::Parked, Cancel::Ended, Cancel::Node] {
        within(scenario(Stage::Style, cancel)).await;
    }
}

#[tokio::test]
async fn held_real_secret_reads_keep_admission_and_node_drain_after_cancellation() {
    for cancel in [Cancel::Typed, Cancel::Parked, Cancel::Ended, Cancel::Node] {
        within(scenario(Stage::Secret, cancel)).await;
    }
}

async fn start(
    fixture: &Fixture,
    handle: &SessionHandle,
    tasks: &TaskTracker,
    stop: &CancellationToken,
) -> (tokio::task::JoinHandle<()>, Arc<Settings>) {
    let settings = Arc::new(Settings::open(
        DataDir::new(fixture.data.path()),
        Arc::clone(&fixture.runtime),
        ParkAfter::Never,
        false,
    ));
    let _changed = settings
        .change(SetNodeSettings {
            metadata: Some(manual()),
            ..SetNodeSettings::default()
        })
        .await
        .unwrap();
    let task = task::Task {
        handle: handle.clone(),
        models: Arc::clone(&fixture.models),
        settings: Arc::clone(&settings),
        cwd: fixture.data.path().to_owned(),
        git: None,
        stop: stop.clone(),
        work: tasks.token(),
    };
    let running = tasks.spawn(async move {
        let client = task.models.title_generator.client.as_ref().unwrap();
        task.run(client).await;
    });
    (running, settings)
}

async fn scenario(stage: Stage, cancel: Cancel) {
    let fixture = Fixture::new().await;
    let (handle, records) = fixture.structured().await;
    handle.send(prompt()).await.unwrap();
    given(&handle).await;
    let tasks = TaskTracker::new();
    let stop = CancellationToken::new();
    let held = fixture.models.blocked_reads.hold(stage);
    let (running, settings) = start(&fixture, &handle, &tasks, &stop).await;
    held.entered().await;
    cancel_title(&handle, &stop, cancel).await;
    running.await.unwrap();
    assert_eq!(
        fixture.models.node_slots.available_permits(),
        PER_NODE - HELD_READS
    );
    assert_eq!(
        fixture
            .models
            .session_slots
            .lock()
            .unwrap()
            .get(&handle.id())
            .unwrap()
            .available_permits(),
        PER_SESSION - HELD_READS
    );
    let replacement = saturate(&fixture).await;
    assert_eq!(held.entries(), HELD_READS);
    assert!(fixture.upstream.seen().is_empty());

    drain(&fixture, tasks, &held, settings).await;
    assert_eq!(fixture.models.node_slots.available_permits(), PER_NODE);
    assert_eq!(
        fixture
            .models
            .session_slots
            .lock()
            .unwrap()
            .get(&handle.id())
            .unwrap()
            .available_permits(),
        PER_SESSION
    );
    assert!(fixture.upstream.seen().is_empty());
    let records = records.await.unwrap();
    let _replacement = replacement.await.unwrap();
    assert!(matches!(
        records.last().unwrap().event,
        SessionEvent::Status { .. }
    ));
    assert!(!records.iter().any(|record| matches!(
        &record.event,
        SessionEvent::ModelCall(_)
            | SessionEvent::Renamed {
                origin: TitleOrigin::Generated,
                ..
            }
    )));
    fixture.stop().await;
}

async fn drain(
    fixture: &Fixture,
    tasks: TaskTracker,
    held: &crate::models::blocking_tests::Hold,
    settings: Arc<Settings>,
) {
    // Use the actual node teardown: a grace-only tracker would finish here.
    fixture.runtime.shutdown().await;
    let store = Store::open(&DataDir::new(fixture.data.path()), StoreConfig::default())
        .await
        .unwrap()
        .store;
    let workspace_labels = crate::workspace_labels::Labels::open(store.clone())
        .await
        .unwrap();
    let closing = CancellationToken::new();
    let draining = crate::shutdown::Teardown {
        workspace_labels,
        runtime: Arc::clone(&fixture.runtime),
        settings,
        tasks: tasks.clone(),
        forwarders: TaskTracker::new(),
        closing: closing.clone(),
        connections: TaskTracker::new(),
        store,
    }
    .run();
    tokio::pin!(draining);
    assert!(futures_util::poll!(draining.as_mut()).is_pending());
    assert!(tasks.is_closed());
    assert!(!closing.is_cancelled());
    held.release();
    draining.await;
    assert!(closing.is_cancelled());
}

async fn saturate(
    fixture: &Fixture,
) -> tokio::task::JoinHandle<Vec<Arc<vessel_proto::EventRecord>>> {
    let remaining = Arc::clone(&fixture.models.node_slots)
        .try_acquire_many_owned(u32::try_from(PER_NODE - HELD_READS).unwrap())
        .unwrap();
    let (replacement, records) = fixture.structured().await;
    replacement.send(prompt()).await.unwrap();
    given(&replacement).await;
    let seed = source::first(&replacement).await.unwrap();
    let settings = manual();
    let request = driver::generate(
        &fixture.models,
        fixture.models.title_generator.client.as_ref().unwrap(),
        &replacement,
        &settings,
        seed,
        context(fixture),
    );
    tokio::pin!(request);
    assert!(matches!(
        futures_util::poll!(request.as_mut()),
        std::task::Poll::Ready(Ok(driver::Outcome::Capacity))
    ));
    drop(remaining);
    records
}
