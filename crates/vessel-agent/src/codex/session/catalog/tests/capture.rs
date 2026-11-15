//! Scoped subscriber records the real handler fields without changing global tracing.

use std::fmt::{self, Write as _};
use std::sync::{Arc, Mutex, PoisonError};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

#[derive(Clone, Default)]
pub(super) struct Capture(Arc<Mutex<String>>);

impl Capture {
    pub(super) fn text(&self) -> String {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

struct Fields<'a>(&'a mut String);
impl Visit for Fields<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let written = writeln!(self.0, "{}={value:?}", field.name());
        assert!(written.is_ok(), "writing the test's String cannot fail");
    }
}

impl Subscriber for Capture {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }
    fn record(&self, _: &Id, _: &Record<'_>) {}
    fn record_follows_from(&self, _: &Id, _: &Id) {}
    fn event(&self, event: &Event<'_>) {
        let mut text = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        event.record(&mut Fields(&mut text));
    }
    fn enter(&self, _: &Id) {}
    fn exit(&self, _: &Id) {}
}
