//! Hold real preparation closures at their read boundary without replacing I/O.

use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::Notify;

#[derive(Clone, Copy)]
pub(super) enum Stage {
    Style,
    Secret,
}

#[derive(Debug, Default)]
pub(super) struct Reads {
    style: Mutex<Option<Arc<Gate>>>,
    secret: Mutex<Option<Arc<Gate>>>,
}
impl Reads {
    fn cell(&self, stage: Stage) -> &Mutex<Option<Arc<Gate>>> {
        match stage {
            Stage::Style => &self.style,
            Stage::Secret => &self.secret,
        }
    }

    pub(super) fn hold(&self, stage: Stage) -> Hold {
        let gate = Arc::new(Gate::default());
        *self.cell(stage).lock().unwrap() = Some(Arc::clone(&gate));
        Hold(gate)
    }

    pub(super) fn gate(&self, stage: Stage) -> Option<Arc<Gate>> {
        self.cell(stage).lock().unwrap().clone()
    }
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Held,
    Released,
}

#[derive(Debug, Default)]
pub(super) struct Gate {
    state: Mutex<State>,
    released: Condvar,
    started: Notify,
    entries: AtomicUsize,
}
impl Gate {
    pub(super) fn enter(&self) {
        let _previous = self.entries.fetch_add(1, Ordering::SeqCst);
        self.started.notify_one();
        let mut state = self.state.lock().unwrap();
        while matches!(*state, State::Held) {
            state = self.released.wait(state).unwrap();
        }
    }
}

pub(super) struct Hold(Arc<Gate>);
impl Hold {
    pub(super) async fn entered(&self) {
        self.0.started.notified().await;
    }

    pub(super) fn entries(&self) -> usize {
        self.0.entries.load(Ordering::SeqCst)
    }

    pub(super) fn release(&self) {
        *self.0.state.lock().unwrap() = State::Released;
        self.0.released.notify_all();
    }
}
impl Drop for Hold {
    fn drop(&mut self) {
        self.release();
    }
}
