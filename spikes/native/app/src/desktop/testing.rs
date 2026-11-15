//! Explicit no-home/no-node process owner for desktop behavior fixtures.

use super::Desktop;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::time::{Duration, Instant};
use vessel_app_link::desktop::{environment::ShellEnvironment, preferences::Coordinator};
use vessel_app_view::desktop::window::WindowState;

const WRITER_WAIT_LIMIT: Duration = Duration::from_secs(10);

struct WriterWake(std::thread::Thread);

impl Wake for WriterWake {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}

/// Await the real blocking writer without pumping GPUI's foreground observers.
/// That separation is the checkpoint-before-UI-observation race under test.
/// std's waker/parking works on every native host without a Linux-only runtime.
#[track_caller]
pub(super) fn wait<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(WriterWake(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    let started = Instant::now();
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => {
                let remaining = WRITER_WAIT_LIMIT.saturating_sub(started.elapsed());
                assert!(!remaining.is_zero(), "blocking writer did not acknowledge");
                std::thread::park_timeout(remaining);
            }
        }
    }
}

pub(super) fn desktop(preferences: Coordinator) -> Desktop {
    Desktop {
        records: Vec::new(),
        saved: None,
        latest: WindowState::default(),
        geometry_publication: std::sync::Arc::new(()),
        writer: None,
        environment: ShellEnvironment::inherited(),
        quit: None,
        window_close: None,
        session: None,
        project: None,
        preferences,
    }
}
