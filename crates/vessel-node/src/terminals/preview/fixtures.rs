//! Test-only authority and peer model; this is never kernel or scheme evidence.

use std::sync::Arc;

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
}

pub(super) struct Control {
    observation: watch::Sender<Observation>,
    pub(super) run: ProcessLifetimeGuard,
}

impl Fixture {
    pub(super) fn current(&self) -> Result<Binding, PreviewError> {
        match &*self.observation.borrow() {
            Observation::Ready(binding) => Ok(binding.clone()),
            Observation::Missing(reason) => Err(PreviewError::Unavailable(*reason)),
        }
    }

    pub(super) async fn changed(&mut self) {
        let _changed = self.observation.changed().await;
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
    let source = Source::Fixture(Fixture {
        observation: receiver,
    });
    (
        PreviewService::new(source, CancellationToken::new(), TaskTracker::new()),
        Control {
            observation: sender,
            run,
        },
    )
}
