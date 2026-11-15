//! Bound the aggregate retained form payload before any batch response effects.
//! Existing comparison/report counters are private and bound different values;
//! this private writer reuses their checked subtraction pattern without making
//! an unrelated shared API or allocating a second serialized payload.
use super::SourceError;
use serde::Serialize;
use std::io::{self, Write};

/// Explicit encoded payload budget; this is not a fixed Rust-heap-size claim.
const MAX_PREFLIGHT_FORM_BYTES: usize = 67_108_864;

pub(super) struct Budget {
    remaining: usize,
    exceeded: bool,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            remaining: MAX_PREFLIGHT_FORM_BYTES,
            exceeded: false,
        }
    }
}
impl Budget {
    pub(super) fn charge(&mut self, form: &impl Serialize) -> Result<(), SourceError> {
        if self.exceeded {
            return Err(SourceError::Budget);
        }
        serde_json::to_writer(&mut *self, form).map_err(|error| {
            if self.exceeded {
                SourceError::Budget
            } else {
                SourceError::Encoding(error)
            }
        })
    }
}
impl Write for Budget {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let Some(remaining) = self.remaining.checked_sub(bytes.len()) else {
            self.exceeded = true;
            return Err(io::Error::other("permit form budget exceeded"));
        };
        self.remaining = remaining;
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
    fn exact_boundary_is_cumulative_and_json_escapes_are_accounted() {
        let mut budget = Budget {
            remaining: 4,
            exceeded: false,
        };
        budget.charge(&"ok").unwrap();
        assert_eq!(budget.remaining, 0);
        assert!(matches!(budget.charge(&""), Err(SourceError::Budget)));
        let mut budget = Budget {
            remaining: 7,
            exceeded: false,
        };
        assert!(matches!(
            budget.charge(&"\u{001f}"),
            Err(SourceError::Budget)
        ));
        let mut poisoned = Budget {
            remaining: 5,
            exceeded: false,
        };
        assert!(matches!(
            poisoned.charge(&"123456789"),
            Err(SourceError::Budget)
        ));
        assert!(matches!(poisoned.charge(&"ok"), Err(SourceError::Budget)));
        let mut budget = Budget {
            remaining: 8,
            exceeded: false,
        };
        budget.charge(&"\u{001f}").unwrap();
        assert_eq!(budget.remaining, 0);
    }
}
