//! The pace of following a node's long reads (setup's output, a clone's
//! progress): each read asks the node to wait for something new, but the
//! node answers at once when it cannot wait (a call forwarded by another
//! node, or all its waits taken), and a loop that asked again straight away
//! would ask back to back for as long as setup or the clone runs. A read
//! that comes back sooner than [`QUIET`] with nothing new waits out the
//! rest of it first (the page's `data/workspace/pace.ts`). The waiting
//! itself is the link's; when to wait is decided here.

use std::time::{Duration, Instant};

/// The least time between two reads that brought nothing new.
pub const QUIET: Duration = Duration::from_secs(1);

/// After a read asked at `asked` that brought nothing new, answered at
/// `now`: how long to wait before asking again (`None`: ask now).
#[must_use]
pub fn quiet_left(asked: Instant, now: Instant) -> Option<Duration> {
    QUIET
        .checked_sub(now.saturating_duration_since(asked))
        .filter(|left| !left.is_zero())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_out_the_rest_of_the_quiet_time_after_a_read_that_came_back_at_once() {
        let asked = Instant::now();
        let now = asked.checked_add(Duration::from_millis(990));
        assert_eq!(
            now.and_then(|now| quiet_left(asked, now)),
            Some(Duration::from_millis(10))
        );
        assert_eq!(quiet_left(asked, asked), Some(QUIET));
    }

    #[test]
    fn does_not_wait_after_a_read_that_waited() {
        let asked = Instant::now();
        let now = asked.checked_add(Duration::from_millis(1001));
        assert_eq!(now.and_then(|now| quiet_left(asked, now)), None);
        let exact = asked.checked_add(QUIET);
        assert_eq!(exact.and_then(|now| quiet_left(asked, now)), None);
    }
}
