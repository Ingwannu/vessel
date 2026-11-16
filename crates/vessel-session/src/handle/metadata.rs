//! One actor order for metadata, separate from process input and native options.
//! Reuse the rename queue instead of introducing a queue for each field.

use super::title::Rename;

/// Metadata changes share the actor's publication and cancellation boundary.
#[derive(Debug)]
pub(crate) enum MetadataChange {
    /// Keep title comparison and its existing acknowledgement in actor order.
    Rename(Rename),
}
