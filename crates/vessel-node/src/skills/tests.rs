//! Cancellation cannot release admission or the serialized writer prematurely.
use super::*;
use vessel_harness::HarnessEnv;
use vessel_proto::skills::SkillNames;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelled_request_keeps_its_writer_and_shutdown_tracking() {
    let root = tempfile::tempdir().unwrap();
    let data = DataDir::new(root.path().join("node"));
    data.prepare().unwrap();
    let config = Config::new(data.clone(), HarnessEnv::new(root.path().join("home")));
    let tasks = TaskTracker::new();
    let stop = CancellationToken::new();
    let skills = Skills::start(&config, tasks.clone(), stop.clone());
    let _fresh = skills.status().await.unwrap();
    let (begun, started) = tokio::sync::oneshot::channel();
    let (release, waiting) = std::sync::mpsc::channel();
    let writer = Arc::clone(&skills);
    let next = SkillSelection::Custom {
        skills: SkillNames::default(),
    };
    let persisted = next.clone();
    let request = tokio::spawn(async move {
        writer
            .run(move |manager, data, durable| {
                begun.send(()).unwrap();
                waiting.recv().unwrap();
                Ok(
                    manager.save(durable, &next, &SkillNames::default(), |selection| {
                        data.write_skills_selection(selection, chrono::Utc::now())
                    })?,
                )
            })
            .await
    });
    started.await.unwrap();
    request.abort();
    assert!(request.await.unwrap_err().is_cancelled());
    assert_eq!(skills.permits.available_permits(), MAX_COMMANDS - 1);
    assert!(
        !tasks.is_empty(),
        "shutdown must still track the detached writer"
    );
    stop.cancel();
    assert!(matches!(skills.status().await, Err(SkillsError::Stopping)));
    release.send(()).unwrap();
    let _closed = tasks.close();
    tasks.wait().await;
    assert_eq!(skills.permits.available_permits(), MAX_COMMANDS);
    assert_eq!(data.read_skills_selection().unwrap(), Some(persisted));
}
