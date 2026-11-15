//! Attempt barriers prove admission precedes spawn and releases after publication.
use super::*;
use std::sync::Mutex;
use tokio::sync::{Notify, Semaphore, mpsc};
use vessel_session::terminals::{TerminalHandle, start_script_admitted};

fn start(
    fixture: &Fixture,
    command: &str,
    service: bool,
) -> ScriptStart<impl Fn() -> bool + Send + Sync + 'static> {
    ScriptStart {
        info: info(script_kind(service), fixture.folder.path()),
        spec: CommandSpec::new("/bin/sh")
            .arg("-c")
            .arg(command)
            .cwd(fixture.folder.path()),
        leftover: Leftover::default(),
        service,
        approved: || true,
        scrollback: Scrollback::DEFAULT,
        shutdown: fixture.shutdown.clone(),
        tasks: fixture.tasks.clone(),
    }
}
struct Hold {
    handle: Arc<Mutex<Option<TerminalHandle>>>,
    published: Arc<AtomicBool>,
    released: Arc<Notify>,
}
impl Drop for Hold {
    fn drop(&mut self) {
        self.published.store(
            self.handle
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .process_lifetime()
                .is_some(),
            Ordering::SeqCst,
        );
        self.released.notify_one();
    }
}
#[tokio::test]
async fn checkpoint_script_start_holds_admission_until_the_process_is_published() {
    within(async {
        let fixture = Fixture::new();
        let barrier = Arc::new(Semaphore::new(0));
        let slot = Arc::new(Mutex::new(None));
        let published = Arc::new(AtomicBool::new(false));
        let released = Arc::new(Notify::new());
        let run = start_script_admitted(start(&fixture, "exec sleep 300", false), {
            let barrier = Arc::clone(&barrier);
            let slot = Arc::clone(&slot);
            let published = Arc::clone(&published);
            let released = Arc::clone(&released);
            move || {
                let barrier = Arc::clone(&barrier);
                let hold = Hold {
                    handle: Arc::clone(&slot),
                    published: Arc::clone(&published),
                    released: Arc::clone(&released),
                };
                async move {
                    let permit = barrier.acquire_owned().await.unwrap();
                    permit.forget();
                    Ok::<_, std::convert::Infallible>(hold)
                }
            }
        });
        *slot.lock().unwrap() = Some(run.clone());
        assert!(run.process_lifetime().is_none());
        barrier.add_permits(1);
        released.notified().await;
        assert!(published.load(Ordering::SeqCst));
        run.kill();
        let _ended = run.ended().await;
    })
    .await;
}
#[tokio::test]
async fn checkpoint_service_reacquires_admission_and_kill_cancels_a_waiting_restart() {
    within(async {
        let fixture = Fixture::new();
        let barrier = Arc::new(Semaphore::new(0));
        let (attempts, mut observed) = mpsc::channel(2);
        let run = start_script_admitted(start(&fixture, "printf x >> attempts; exit 1", true), {
            let barrier = Arc::clone(&barrier);
            move || {
                let barrier = Arc::clone(&barrier);
                let attempts = attempts.clone();
                async move {
                    attempts.send(()).await.unwrap();
                    let permit = barrier.acquire_owned().await.unwrap();
                    permit.forget();
                    Ok::<_, std::convert::Infallible>(())
                }
            }
        });
        observed.recv().await.unwrap();
        assert!(!fixture.folder.path().join("attempts").exists());
        barrier.add_permits(1);
        observed.recv().await.unwrap();
        assert_eq!(
            std::fs::read(fixture.folder.path().join("attempts")).unwrap(),
            b"x"
        );
        assert!(run.process_lifetime().is_none());
        run.kill();
        let _ended = run.ended().await;
        assert_eq!(
            std::fs::read(fixture.folder.path().join("attempts")).unwrap(),
            b"x"
        );
    })
    .await;
}
#[tokio::test]
async fn checkpoint_refused_script_admission_never_executes_the_command() {
    within(async {
        let fixture = Fixture::new();
        let run = start_script_admitted(start(&fixture, "printf x > unexpected", false), || {
            std::future::ready(Err::<(), _>(std::io::Error::other("admission refused")))
        });
        let _ended = run.ended().await;
        assert!(matches!(run.info().status, SessionStatus::Failed { .. }));
        assert!(!fixture.folder.path().join("unexpected").exists());
    })
    .await;
}
