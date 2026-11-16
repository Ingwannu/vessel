//! Real mutex/admission futures, with event-driven scheduling and no sleeps.
use super::*;

const DEADLINE: std::time::Duration = std::time::Duration::from_secs(10);
#[tokio::test]
async fn one_project_waits_while_another_can_write_and_releasing_all_frees_the_map() {
    vessel_testkit::within(DEADLINE, async {
        let writers = Writers::default();
        let stop = CancellationToken::new();
        let project = ProjectId::random();
        let first = writers.acquire(project, &stop).await.unwrap();
        let mut second = Box::pin(writers.acquire(project, &stop));
        assert!(futures_util::poll!(&mut second).is_pending());
        let other = writers.acquire(ProjectId::random(), &stop).await.unwrap();
        drop(first);
        let second = second.await.unwrap();
        drop((other, second));
        assert_eq!(writers.admitted.available_permits(), MAX_WRITERS);
        let next = writers.acquire(ProjectId::random(), &stop).await.unwrap();
        assert_eq!(writers.projects.lock().unwrap().len(), 1);
        drop(next);
    })
    .await
    .expect("mutation lease futures should settle after releasing their barriers");
}
#[tokio::test]
async fn cancelled_waiters_never_enter_and_release_their_admission() {
    vessel_testkit::within(DEADLINE, async {
        let writers = Writers::default();
        let stop = CancellationToken::new();
        let project = ProjectId::random();
        let first = writers
            .acquire(project, &CancellationToken::new())
            .await
            .unwrap();
        let mut waiting = Box::pin(writers.acquire(project, &stop));
        assert!(futures_util::poll!(&mut waiting).is_pending());
        stop.cancel();
        assert!(matches!(waiting.await, Err(WriteAdmission::Closed)));
        drop(first);
        assert_eq!(writers.admitted.available_permits(), MAX_WRITERS);
    })
    .await
    .expect("mutation lease futures should settle after releasing their barriers");
}
#[tokio::test]
async fn a_full_writer_reservation_refuses_before_queueing_more_work() {
    vessel_testkit::within(DEADLINE, async {
        let writers = Writers::default();
        let stop = CancellationToken::new();
        let mut active = Vec::new();
        for _ in 0..MAX_WRITERS {
            active.push(writers.acquire(ProjectId::random(), &stop).await.unwrap());
        }
        assert!(matches!(
            writers.acquire(ProjectId::random(), &stop).await,
            Err(WriteAdmission::Busy)
        ));
        drop(active);
        assert_eq!(writers.admitted.available_permits(), MAX_WRITERS);
    })
    .await
    .expect("mutation lease futures should settle after releasing their barriers");
}

#[tokio::test]
async fn same_project_waiters_keep_the_existing_mutex_fifo_order() {
    vessel_testkit::within(DEADLINE, async {
        let writers = Writers::default();
        let stop = CancellationToken::new();
        let project = ProjectId::random();
        let first = writers.acquire(project, &stop).await.unwrap();
        let mut second = Box::pin(writers.acquire(project, &stop));
        let mut third = Box::pin(writers.acquire(project, &stop));
        assert!(futures_util::poll!(&mut second).is_pending());
        assert!(futures_util::poll!(&mut third).is_pending());
        drop(first);
        let second = second.await.unwrap();
        assert!(futures_util::poll!(&mut third).is_pending());
        drop(second);
        drop(third.await.unwrap());
    })
    .await
    .expect("mutation lease futures should settle after releasing their barriers");
}
