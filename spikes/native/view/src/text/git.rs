//! Git's sentences, in English and Korean. Domain errors stay typed;
//! their `Display` is never the screen's primary explanation. Git's own
//! output remains separate detail beneath the explanation and next step.

mod calls;
mod failures;
mod header;
mod metadata;
mod pending;
mod presentation;
mod screen;
mod status;
mod success;

pub use failures::GitFailureText;
pub use header::{GitHashText, GitHeaderText};
pub use metadata::GitStashText;
pub use screen::GitLabels;
pub use success::{GitSuccessCommits, GitSuccessText};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod value_tests;

#[cfg(test)]
mod call_tests;

#[cfg(test)]
mod metadata_tests;
#[cfg(test)]
mod presentation_tests;
#[cfg(test)]
mod success_tests;

#[cfg(test)]
mod header_tests;
