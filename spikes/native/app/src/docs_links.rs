//! Product-owned documentation destinations. A missing destination hides its
//! affordance: another product's documentation cannot explain Vessel's UI.

/// A trusted static destination owned by this module, never node-provided text.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Destination {
    url: &'static str,
}
impl Destination {
    pub(crate) const fn url(self) -> &'static str {
        self.url
    }
}

/// Vessel user docs do not exist yet. Set this once they cover schedules;
/// the screen already uses the shared external-link control for `Some`.
pub(crate) const SCHEDULES: Option<Destination> = None;
