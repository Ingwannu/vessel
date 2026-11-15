//! Test-only authority and peer model; this is never kernel or scheme evidence.

use std::{
    net::TcpStream,
    sync::{Arc, Condvar, Mutex},
};

use tokio::{sync::watch, time::Instant};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_proc::{ProcessId, ProcessLifetimeGuard};

use super::{
    MissingPreviewProof, PreviewContext, PreviewError, PreviewOrigin, PreviewService,
    source::{Authority, Binding, Source},
};

#[derive(Clone)]
enum Observation {
    Ready(Binding),
    Missing(MissingPreviewProof),
}

#[derive(Clone)]
pub(super) struct Fixture {
    observation: watch::Receiver<Observation>,
    pub(super) inspection: Arc<Inspection>,
}

pub(super) struct Control {
    observation: watch::Sender<Observation>,
    pub(super) inspection: Arc<Inspection>,
    pub(super) run: ProcessLifetimeGuard,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Gate {
    Open,
    Closed,
}

pub(super) struct Inspection {
    gate: Mutex<Gate>,
    released: Condvar,
    pub(super) entered: tokio::sync::Notify,
}

impl Inspection {
    pub(super) fn block(&self) {
        *self.gate.lock().unwrap() = Gate::Closed;
    }

    pub(super) fn release(&self) {
        *self.gate.lock().unwrap() = Gate::Open;
        self.released.notify_all();
    }

    fn inspect(&self) {
        let guard = self.gate.lock().unwrap();
        self.entered.notify_one();
        let _guard = self
            .released
            .wait_while(guard, |state| *state == Gate::Closed)
            .unwrap();
    }
}

impl Drop for Control {
    fn drop(&mut self) {
        self.inspection.release();
    }
}

impl Fixture {
    pub(super) fn current(&self) -> Result<Binding, PreviewError> {
        match &*self.observation.borrow() {
            Observation::Ready(binding) => Ok(binding.clone()),
            Observation::Missing(reason) => Err(PreviewError::Unavailable(*reason)),
        }
    }

    pub(super) async fn changed(&mut self) -> Result<(), watch::error::RecvError> {
        self.observation.changed().await
    }

    pub(super) fn inspect(
        &self,
        socket: &TcpStream,
        expected: &Binding,
    ) -> Result<(), PreviewError> {
        self.inspection.inspect();
        if socket.peer_addr().map_err(PreviewError::Socket)? != expected.origin.endpoint() {
            return Err(PreviewError::Origin);
        }
        let current = self.current()?;
        if !expected.same_binding(&current) {
            return Err(PreviewError::Changed);
        }
        current.validate()
    }
}

impl Control {
    pub(super) fn refresh(&self, observed: Instant) {
        self.observation.send_modify(|state| {
            if let Observation::Ready(binding) = state {
                binding.observed = observed;
            }
        });
    }

    pub(super) fn replace_run(&self) -> ProcessLifetimeGuard {
        let next = ProcessLifetimeGuard::begin(self.run.observer().pid());
        self.observation.send_modify(|state| {
            if let Observation::Ready(binding) = state {
                binding.run = next.observer();
                binding.observed = Instant::now();
            }
        });
        next
    }

    pub(super) fn replace_authority(&self) {
        self.observation.send_modify(|state| {
            if let Observation::Ready(binding) = state {
                binding.authority = Arc::new(Authority);
            }
        });
    }

    pub(super) fn missing(&self, reason: MissingPreviewProof) {
        let _old = self.observation.send_replace(Observation::Missing(reason));
    }
}

pub(super) fn service(
    context: &PreviewContext,
    origin: PreviewOrigin,
) -> (PreviewService, Control) {
    let run = ProcessLifetimeGuard::begin(ProcessId::MIN);
    let binding = Binding {
        scope: context.scope,
        origin,
        run: run.observer(),
        observed: Instant::now(),
        authority: Arc::new(Authority),
    };
    let (sender, receiver) = watch::channel(Observation::Ready(binding));
    let inspection = Arc::new(Inspection {
        gate: Mutex::new(Gate::Open),
        released: Condvar::new(),
        entered: tokio::sync::Notify::new(),
    });
    let source = Source::Fixture(Fixture {
        observation: receiver,
        inspection: Arc::clone(&inspection),
    });
    (
        PreviewService::new(source, CancellationToken::new(), TaskTracker::new()),
        Control {
            observation: sender,
            inspection,
            run,
        },
    )
}
