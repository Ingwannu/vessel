//! Controlled ownership barriers, not a browser engine or OS cleanup certificate.
use super::*;
use std::time::Duration;
use tokio::sync::mpsc;
const FIXTURE_DEADLINE: Duration = Duration::from_secs(20);
const CALL_DEADLINE: Duration = Duration::from_secs(2);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    A,
    B,
}
#[derive(Debug, Clone, Copy)]
enum Command {
    Apply,
    Fail,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Event {
    Activated(Target),
    Applied(Target),
    StopStarted,
    Stopped,
}
#[derive(Debug, thiserror::Error)]
#[error("controlled input failure")]
struct Fault;
#[derive(Clone, Copy)]
enum StopOutcome {
    Success,
    Refused,
}
struct Mock {
    events: mpsc::Sender<Event>,
    apply: Arc<Semaphore>,
    stop: Arc<Semaphore>,
    outcome: StopOutcome,
}
impl Driver for Mock {
    type Target = Target;
    type Command = Command;
    type Error = Fault;
    async fn activate(&self, target: &Target) -> Result<(), Fault> {
        self.events.send(Event::Activated(*target)).await.unwrap();
        Ok(())
    }
    async fn apply(&self, target: &Target, command: Command) -> Result<(), Fault> {
        let permit = self.apply.acquire().await.unwrap();
        permit.forget();
        self.events.send(Event::Applied(*target)).await.unwrap();
        match command {
            Command::Apply => Ok(()),
            Command::Fail => Err(Fault),
        }
    }
    async fn stop_and_drain(&self) -> Result<(), Fault> {
        self.events.send(Event::StopStarted).await.unwrap();
        let permit = self.stop.acquire().await.unwrap();
        permit.forget();
        self.events.send(Event::Stopped).await.unwrap();
        match self.outcome {
            StopOutcome::Success => Ok(()),
            StopOutcome::Refused => Err(Fault),
        }
    }
}
struct Fixture {
    lane: InputLane<Mock>,
    events: mpsc::Receiver<Event>,
    apply: Arc<Semaphore>,
    stop: Arc<Semaphore>,
}
fn fixture(capacity: usize) -> Fixture {
    fixture_with_stop(capacity, StopOutcome::Success)
}
fn fixture_with_stop(capacity: usize, outcome: StopOutcome) -> Fixture {
    const EVENT_CAPACITY: usize = 16;
    let (events, receiver) = mpsc::channel(EVENT_CAPACITY);
    let apply = Arc::new(Semaphore::new(0));
    let stop = Arc::new(Semaphore::new(0));
    let driver = Mock {
        events,
        apply: Arc::clone(&apply),
        stop: Arc::clone(&stop),
        outcome,
    };
    let lane = if capacity == MAX_INPUT_TRANSACTIONS {
        InputLane::new(driver)
    } else {
        InputLane::with_capacity(driver, capacity)
    };
    Fixture {
        lane,
        events: receiver,
        apply,
        stop,
    }
}
#[tokio::test]
async fn activation_and_entire_input_transaction_stay_serial_across_targets() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let mut f = fixture(MAX_INPUT_TRANSACTIONS);
        let a = f.lane.clone();
        let b = f.lane.clone();
        let first = tokio::spawn(async move { a.run(Target::A, Command::Apply).await });
        assert_eq!(f.events.recv().await, Some(Event::Activated(Target::A)));
        let second = tokio::spawn(async move { b.run(Target::B, Command::Apply).await });
        tokio::task::yield_now().await;
        assert!(f.events.try_recv().is_err());
        f.apply.add_permits(1);
        first.await.unwrap().unwrap();
        assert_eq!(f.events.recv().await, Some(Event::Applied(Target::A)));
        assert_eq!(f.events.recv().await, Some(Event::Activated(Target::B)));
        f.apply.add_permits(1);
        second.await.unwrap().unwrap();
        assert_eq!(f.events.recv().await, Some(Event::Applied(Target::B)));
        f.stop.add_permits(1);
        f.lane.shutdown().await.unwrap();
        assert_eq!(f.events.recv().await, Some(Event::StopStarted));
        assert_eq!(f.events.recv().await, Some(Event::Stopped));
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn caller_abort_holds_capacity_until_real_driver_drain_and_never_reopens() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let mut f = fixture(1);
        let lane = f.lane.clone();
        let caller = tokio::spawn(async move { lane.run(Target::A, Command::Apply).await });
        assert_eq!(f.events.recv().await, Some(Event::Activated(Target::A)));
        assert!(matches!(
            f.lane.run(Target::B, Command::Apply).await,
            Err(InputError::Busy)
        ));
        caller.abort();
        assert!(caller.await.unwrap_err().is_cancelled());
        assert_eq!(f.events.recv().await, Some(Event::StopStarted));
        assert_eq!(f.lane.0.capacity.available_permits(), 0);
        assert!(matches!(
            f.lane.run(Target::B, Command::Apply).await,
            Err(InputError::Closed)
        ));
        f.stop.add_permits(1);
        f.lane.shutdown().await.unwrap();
        assert_eq!(f.events.recv().await, Some(Event::Stopped));
        assert_eq!(f.lane.0.capacity.available_permits(), 1);
        assert!(matches!(
            f.lane.run(Target::B, Command::Apply).await,
            Err(InputError::Closed)
        ));
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn delivered_timeout_is_uncertain_and_does_not_release_worker_early() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let mut f = fixture(1);
        let lane = f.lane.clone();
        let caller = tokio::spawn(async move {
            lane.run_with_budget(Target::A, Command::Apply, CALL_DEADLINE)
                .await
        });
        assert_eq!(f.events.recv().await, Some(Event::Activated(Target::A)));
        assert!(matches!(
            caller.await.unwrap(),
            Err(InputError::TimedOut {
                effect: BrowserEffect::Uncertain,
                ..
            })
        ));
        assert_eq!(f.events.recv().await, Some(Event::StopStarted));
        assert_eq!(f.lane.0.capacity.available_permits(), 0);
        f.stop.add_permits(1);
        f.lane.shutdown().await.unwrap();
        assert_eq!(f.events.recv().await, Some(Event::Stopped));
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn backend_error_is_retained_after_actual_profile_stop_not_retried() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let mut f = fixture(1);
        let lane = f.lane.clone();
        let caller = tokio::spawn(async move { lane.run(Target::A, Command::Fail).await });
        assert_eq!(f.events.recv().await, Some(Event::Activated(Target::A)));
        f.apply.add_permits(1);
        assert_eq!(f.events.recv().await, Some(Event::Applied(Target::A)));
        assert_eq!(f.events.recv().await, Some(Event::StopStarted));
        assert!(!caller.is_finished());
        f.stop.add_permits(1);
        assert!(matches!(
            caller.await.unwrap(),
            Err(InputError::Uncertain { .. })
        ));
        f.lane.shutdown().await.unwrap();
        assert_eq!(f.events.recv().await, Some(Event::Stopped));
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn timed_out_queued_attempt_cannot_activate_after_preceding_target_finishes() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let mut f = fixture(2);
        let lane = f.lane.clone();
        let first = tokio::spawn(async move { lane.run(Target::A, Command::Apply).await });
        assert_eq!(f.events.recv().await, Some(Event::Activated(Target::A)));
        let result = f
            .lane
            .run_with_budget(Target::B, Command::Apply, CALL_DEADLINE)
            .await;
        assert!(matches!(
            result,
            Err(InputError::TimedOut {
                effect: BrowserEffect::NotDelivered,
                ..
            })
        ));
        f.apply.add_permits(1);
        first.await.unwrap().unwrap();
        assert_eq!(f.events.recv().await, Some(Event::Applied(Target::A)));
        f.stop.add_permits(1);
        f.lane.shutdown().await.unwrap();
        // Shutdown joins both workers, so no unseen late B activation remains.
        assert_eq!(f.events.recv().await, Some(Event::StopStarted));
        assert_eq!(f.events.recv().await, Some(Event::Stopped));
        assert!(f.events.try_recv().is_err());
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn cancelled_shutdown_waiter_cannot_cancel_or_repeat_actual_stop() {
    tokio::time::timeout(FIXTURE_DEADLINE, async {
        let mut f = fixture_with_stop(1, StopOutcome::Refused);
        let lane = f.lane.clone();
        let waiter = tokio::spawn(async move { lane.shutdown().await });
        assert_eq!(f.events.recv().await, Some(Event::StopStarted));
        waiter.abort();
        assert!(waiter.await.unwrap_err().is_cancelled());
        f.stop.add_permits(1);
        let first = f.lane.shutdown().await.unwrap_err();
        assert_eq!(f.events.recv().await, Some(Event::Stopped));
        let second = f.lane.shutdown().await.unwrap_err();
        assert!(Arc::ptr_eq(&first, &second));
        assert!(f.events.try_recv().is_err());
        assert!(matches!(
            f.lane.run(Target::A, Command::Apply).await,
            Err(InputError::Closed)
        ));
    })
    .await
    .unwrap();
}
