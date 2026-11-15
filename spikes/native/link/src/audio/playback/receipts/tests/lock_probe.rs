//! A lock-regression probe runs in a child so a reintroduced deadlock has a finite teardown.

use super::*;
use std::{
    io::Read as _,
    process::{Child, Command, Stdio},
};

const CHILD_FLAG: &str = "VESSEL_VOICE_RECEIPT_LOCK_PROBE";
const TEST_NAME: &str = "audio::playback::receipts::tests::lock_probe::receipt_updates_do_not_reenter_their_watch_write_lock";
const PROBE_DEADLINE: Duration = Duration::from_secs(5);
const POLL_STEP: Duration = Duration::from_millis(10);
const MAX_HARNESS_OUTPUT: u64 = 4_096;

struct Probe(Child);
impl Drop for Probe {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(None)) {
            let _kill = self.0.kill();
            let _reaped = self.0.wait();
        }
    }
}

#[test]
fn receipt_updates_do_not_reenter_their_watch_write_lock() {
    if std::env::var_os(CHILD_FLAG).is_some() {
        exercise_transitions();
        return;
    }
    let mut child = Probe(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST_NAME, "--test-threads=1"])
            .env(CHILD_FLAG, "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now().checked_add(PROBE_DEADLINE).unwrap();
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(
                status.success(),
                "receipt transition child failed: {status}"
            );
            let mut output = String::new();
            child
                .0
                .stdout
                .take()
                .unwrap()
                .take(MAX_HARNESS_OUTPUT)
                .read_to_string(&mut output)
                .unwrap();
            // A renamed/wrong exact filter running zero tests is not proof.
            assert!(output.contains("1 passed; 0 failed"), "{output}");
            return;
        }
        assert!(
            Instant::now() < deadline,
            "receipt transition child exceeded its deadline"
        );
        std::thread::sleep(POLL_STEP);
    }
}

fn exercise_transitions() {
    let (mut book, _fence) = setup();
    let played = book.submit(packet(&[0, 0])).unwrap();
    let mut reader = book.begin(played.lease()).unwrap().unwrap();
    assert!(reader.next().unwrap().unwrap().last());
    let now = Instant::now();
    book.scheduled(played.lease(), now, Some(Duration::ZERO))
        .unwrap();
    book.advance(now);
    assert_eq!(*played.status().borrow(), State::Played);
    let failed = book.submit(packet(&[0, 0])).unwrap();
    let mut reader = book.begin(failed.lease()).unwrap().unwrap();
    assert_eq!(
        book.scheduled(failed.lease(), now, None),
        Err(Refusal::Clock)
    );
    assert!(reader.stopped());
    assert!(matches!(reader.next(), Err(Refusal::Stopped)));
    assert_eq!(*failed.status().borrow(), State::Failed);
    book.advance(now);
    let interrupted = book.submit(packet(&[0, 0])).unwrap();
    book.interrupt_all();
    assert_eq!(*interrupted.status().borrow(), State::Interrupted);
}
