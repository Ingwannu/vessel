//! Compatibility path for the shared local client-book label.
//! Keeping consumer tests here also guards the distinction from ASCII aliases.

pub use vessel_proto::book_values::{EmptyLabel, Label};

#[cfg(test)]
mod tests;
