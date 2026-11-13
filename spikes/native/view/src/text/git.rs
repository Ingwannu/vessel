//! Git's sentences, in English and Korean. Domain errors stay typed;
//! their `Display` is never the screen's primary explanation. Git's own
//! output remains separate detail beneath the explanation and next step.

mod failures;
mod status;

pub use failures::GitFailureText;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod value_tests;
