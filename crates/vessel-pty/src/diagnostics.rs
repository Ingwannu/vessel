//! Suppress upstream command-bearing records only during a private backend call.
use tracing::{Dispatch, Event, Metadata, Subscriber};
use tracing_core::{span, subscriber::Interest};
use vessel_proc::{CommandDiagnostics, CommandSpec};

/// Keep the tested boundary and actual backend call together on this thread.
pub(crate) fn spawn_backend(
    slave: &dyn portable_pty::SlavePty,
    spec: &CommandSpec,
) -> Result<Box<dyn portable_pty::Child + Send + Sync>, crate::PtyError> {
    #[cfg(test)]
    tests::backend_probe();
    slave
        .spawn_command(crate::command::command_builder(spec))
        .map_err(|error| crate::PtyError::Spawn {
            program: spec.program.display().to_string(),
            message: error.to_string(),
        })
}

pub(crate) fn spawn<T>(spec: &CommandSpec, run: impl FnOnce() -> T) -> T {
    match spec.diagnostic_policy() {
        CommandDiagnostics::Standard => run(),
        CommandDiagnostics::Redacted => {
            let parent = tracing::dispatcher::get_default(Clone::clone);
            // The default is thread-local. Keep the entire backend spawn
            // synchronous on THIS thread; never detach or await inside run.
            // The backend-boundary test probe checks the actual call's thread.
            tracing::subscriber::with_default(PrivateBackend(parent), run)
        }
    }
}

fn backend(target: &str) -> bool {
    target == "portable_pty" || target.starts_with("portable_pty::")
}

// Subscriber is tracing's open extension boundary. Forward every safe event and
// span operation to the original dispatcher, without installing a global logger.
struct PrivateBackend(Dispatch);
impl Subscriber for PrivateBackend {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        !backend(metadata.target()) && self.0.enabled(metadata)
    }
    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        if backend(metadata.target()) {
            Interest::never()
        } else {
            self.0.register_callsite(metadata)
        }
    }
    fn new_span(&self, attributes: &span::Attributes<'_>) -> span::Id {
        self.0.new_span(attributes)
    }
    fn record(&self, id: &span::Id, values: &span::Record<'_>) {
        self.0.record(id, values);
    }
    fn record_follows_from(&self, id: &span::Id, follows: &span::Id) {
        self.0.record_follows_from(id, follows);
    }
    fn event(&self, event: &Event<'_>) {
        if !backend(event.metadata().target()) {
            self.0.event(event);
        }
    }
    fn enter(&self, id: &span::Id) {
        self.0.enter(id);
    }
    fn exit(&self, id: &span::Id) {
        self.0.exit(id);
    }
    fn clone_span(&self, id: &span::Id) -> span::Id {
        self.0.clone_span(id)
    }
    fn try_close(&self, id: span::Id) -> bool {
        self.0.try_close(id)
    }
    fn current_span(&self) -> span::Current {
        self.0.current_span()
    }
}

#[cfg(test)]
pub(crate) mod tests;
