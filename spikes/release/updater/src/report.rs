//! An error as one line for the log: the error and every cause under it.
//!
//! [Decision Log]
//! - Purpose: when an update fails on the owner's Windows the log must say
//!   why, down to the operating system's code (0.0.0.0.3's first real
//!   update failed and nothing said where).
//! - Constraint: the errors' `Display` is one short sentence each and the
//!   cause sits in `source()` (`StageError::Write` holds the `io::Error`);
//!   `tracing`'s `%error` prints the first sentence only.
//! - Choice: join every `Display` down the `source()` chain with ": ".
//!   An `io::Error` prints its OS code itself ("Access is denied. (os
//!   error 5)"); a reqwest error never carries the request's address here
//!   (`FetchError::unreachable` drops it).

use std::fmt::Write as _;

/// `error` and its causes, outermost first: `"a: b: c"`.
#[must_use]
pub fn chain(error: &(dyn std::error::Error + 'static)) -> String {
    let mut line = error.to_string();
    let mut cause = error.source();
    while let Some(next) = cause {
        let _written = write!(line, ": {next}");
        cause = next.source();
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage::StageError;

    #[test]
    fn the_os_code_under_a_short_error_is_kept() {
        let denied = std::io::Error::from_raw_os_error(13);
        let error = StageError::Write(denied);
        let line = chain(&error);
        assert!(
            line.starts_with("the update could not be written beside the app: "),
            "{line}"
        );
        assert!(line.contains("os error 13"), "{line}");
    }
}
