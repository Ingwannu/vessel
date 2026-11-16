//! Tiny per-crate test subscriber: records only this synchronous launch scope.
use std::fmt::Write as _;
use std::sync::{Arc, Mutex};
use tracing::{
    Event, Metadata, Subscriber,
    field::{Field, Visit},
    span::{Attributes, Id, Record},
};

#[derive(Clone, Default)]
pub(super) struct Capture(Arc<Mutex<String>>);
impl Capture {
    pub(super) fn text(&self) -> String {
        self.0.lock().unwrap().clone()
    }
}
struct Fields<'a>(&'a mut String);
impl Visit for Fields<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let _written = write!(self.0, "{}={value:?} ", field.name());
    }
}
impl Subscriber for Capture {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _attributes: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }
    fn record(&self, _span: &Id, _values: &Record<'_>) {}
    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}
    fn event(&self, event: &Event<'_>) {
        let mut text = self.0.lock().unwrap();
        event.record(&mut Fields(&mut text));
    }
    fn enter(&self, _span: &Id) {}
    fn exit(&self, _span: &Id) {}
}
