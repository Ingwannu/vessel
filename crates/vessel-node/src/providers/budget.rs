//! Check actual encoded reader projections without allocating a frame.
//! Arc-shared thinking choices save retained memory but serialize per model;
//! count that fan-out before a Ready value enters the cache or a list is sent.

use serde::Serialize;
use std::io::{self, Write};

/// Leave half the canonical frame for envelope/response metadata. This is a
/// projection bound, not a second transport limit or a truncation threshold.
pub(super) const MAX_PROVIDER_REPLY_BYTES: usize = match vessel_rpc::MAX_FRAME.get().checked_div(2)
{
    Some(bytes) => bytes,
    None => 0,
};

pub(super) fn fits(value: &impl Serialize) -> bool {
    fits_with_limit(value, MAX_PROVIDER_REPLY_BYTES)
}

fn fits_with_limit(value: &impl Serialize, remaining: usize) -> bool {
    serde_json::to_writer(Counter { remaining }, value).is_ok()
}

struct Counter {
    remaining: usize,
}

impl Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.remaining = self
            .remaining
            .checked_sub(bytes.len())
            .ok_or_else(|| io::Error::other("provider reply exceeds its byte budget"))?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoding_budget_counts_json_escaping_and_exact_boundary_without_a_buffer() {
        let text = "quotes \" and slashes \\ and lines \n";
        let encoded = serde_json::to_vec(text).unwrap();
        assert!(encoded.len() > text.len());
        assert!(fits_with_limit(&text, encoded.len()));
        assert!(!fits_with_limit(&text, encoded.len() - 1));
    }
}
