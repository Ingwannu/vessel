//! No sleeps: barriers prove task lifetime, queue bounds and shutdown ordering.
use super::*;

#[tokio::test]
async fn losing_the_receiver_does_not_drop_running_cleanup_or_release_its_reservation_early() {
    let jobs = Jobs::default();
    let tasks = TaskTracker::new();
    let stop = CancellationToken::new();
    let (started, running) = oneshot::channel();
    let (finish, cleanup) = oneshot::channel();
    let pending = jobs
        .submit(&tasks, stop, async move {
            started.send(()).unwrap();
            cleanup.await.unwrap();
        })
        .unwrap();
    running.await.unwrap();
    drop(pending);
    assert_eq!(jobs.admitted.available_permits(), MAX_ADMITTED - 1);
    assert_eq!(jobs.executing.available_permits(), MAX_EXECUTING - 1);
    assert!(tasks.close());
    assert!(!tasks.is_empty());
    finish.send(()).unwrap();
    tasks.wait().await;
    assert_eq!(jobs.admitted.available_permits(), MAX_ADMITTED);
    assert_eq!(jobs.executing.available_permits(), MAX_EXECUTING);
}

#[tokio::test]
async fn the_queue_refuses_at_64_and_shutdown_does_not_start_waiting_operations() {
    let jobs = Jobs::default();
    let tasks = TaskTracker::new();
    let stop = CancellationToken::new();
    let (started, mut running) = tokio::sync::mpsc::unbounded_channel();
    let mut pending = Vec::new();
    for index in 0..MAX_ADMITTED {
        let stop_work = stop.clone();
        let started = started.clone();
        pending.push(
            jobs.submit(&tasks, stop.clone(), async move {
                started.send(index).unwrap();
                stop_work.cancelled().await;
                index
            })
            .unwrap(),
        );
    }
    drop(started);
    for _slot in 0..MAX_EXECUTING {
        let _index = running.recv().await.unwrap();
    }
    assert!(matches!(
        jobs.submit(&tasks, stop.clone(), async {}),
        Err(JobError::Busy)
    ));
    assert_eq!(jobs.executing.available_permits(), 0);
    stop.cancel();
    let mut completed = 0;
    let mut closed = 0;
    for answer in pending {
        match answer.result().await {
            Ok(_) => completed += 1,
            Err(JobError::Closed) => closed += 1,
            Err(error @ (JobError::Busy | JobError::Worker(_))) => panic!("{error}"),
        }
    }
    assert_eq!(
        (completed, closed),
        (MAX_EXECUTING, MAX_ADMITTED - MAX_EXECUTING)
    );
    assert!(running.recv().await.is_none());
    assert!(tasks.close());
    tasks.wait().await;
    assert_eq!(jobs.admitted.available_permits(), MAX_ADMITTED);
    assert!(matches!(
        jobs.submit(&tasks, stop, async {}),
        Err(JobError::Closed)
    ));
}

#[tokio::test]
async fn shutdown_waits_for_started_work_to_finish_its_cleanup() {
    let jobs = Jobs::default();
    let tasks = TaskTracker::new();
    let stop = CancellationToken::new();
    let token = stop.clone();
    let (started, running) = oneshot::channel();
    let (finish, cleanup) = oneshot::channel();
    let answer = jobs
        .submit(&tasks, stop.clone(), async move {
            started.send(()).unwrap();
            token.cancelled().await;
            cleanup.await.unwrap();
            "cleaned"
        })
        .unwrap();
    running.await.unwrap();
    stop.cancel();
    assert!(tasks.close());
    assert!(!tasks.is_empty());
    assert_eq!(jobs.executing.available_permits(), MAX_EXECUTING - 1);
    finish.send(()).unwrap();
    assert_eq!(answer.result().await.unwrap(), "cleaned");
    tasks.wait().await;
    assert_eq!(jobs.admitted.available_permits(), MAX_ADMITTED);
}
