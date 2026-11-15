//! Submission-order text with bounded aggregate storage, without provider I/O.
//!
//! Pending slots stay empty in partial joins. Preserve each segment's spacing,
//! then trim the complete join using JavaScript's whitespace set, as Paseo does.

use super::budget::PendingCommits;
use vessel_proto::{dictation::Transcript, workspace::trim_project_text};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CommitIndex(usize);

#[derive(Debug, thiserror::Error)]
enum TextError {
    #[error("the aggregate dictation transcript exceeds its byte bound")]
    TooLong,
    #[error("the joined transcript could not be trimmed at a valid boundary")]
    Boundary,
    #[error("a dictation result did not name a pending commit")]
    Commit,
    #[error("the joined dictation transcript failed validation")]
    Transcript(#[source] vessel_proto::dictation::DictationDecodeError),
}

#[derive(Default)]
struct Text {
    // No Debug: even independently redacted transcript values should not gain
    // a diagnostic container that might later expose newly added fields.
    slots: Vec<Option<Transcript>>,
    retained: usize,
    pending: usize,
}

impl Text {
    fn reserve(&mut self) -> Result<CommitIndex, TextError> {
        let index = CommitIndex(self.slots.len());
        let separators = self.slots.len();
        if self.retained.saturating_add(separators) > Transcript::MAX_BYTES {
            return Err(TextError::TooLong);
        }
        self.slots.push(None);
        self.pending = self.pending.saturating_add(1);
        Ok(index)
    }

    fn complete(&mut self, index: CommitIndex, text: Transcript) -> Result<(), TextError> {
        let separators = self.slots.len().saturating_sub(1);
        let slot = self.slots.get_mut(index.0).ok_or(TextError::Commit)?;
        if slot.is_some() {
            return Err(TextError::Commit);
        }
        let retained = self.retained.saturating_add(text.as_str().len());
        if retained.saturating_add(separators) > Transcript::MAX_BYTES {
            return Err(TextError::TooLong);
        }
        *slot = Some(text);
        self.retained = retained;
        self.pending = self.pending.saturating_sub(1);
        Ok(())
    }

    fn join(&self) -> Result<Transcript, TextError> {
        let mut joined = String::with_capacity(
            self.retained
                .saturating_add(self.slots.len().saturating_sub(1)),
        );
        for (index, slot) in self.slots.iter().enumerate() {
            if index != 0 {
                joined.push(' ');
            }
            if let Some(text) = slot {
                joined.push_str(text.as_str());
            }
        }
        // Truncate/drain in place so one event allocates its output only once.
        let trimmed = trim_project_text(&joined);
        let start = joined.find(trimmed).ok_or(TextError::Boundary)?;
        let end = start.saturating_add(trimmed.len());
        joined.truncate(end);
        drop(joined.drain(..start));
        Transcript::try_from(joined).map_err(TextError::Transcript)
    }

    fn pending(&self) -> PendingCommits {
        PendingCommits::new(self.pending)
    }

    fn finished(&self) -> bool {
        self.pending == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn text(value: &str) -> Transcript {
        Transcript::try_from(value.to_owned()).unwrap()
    }

    #[test]
    fn out_of_order_results_join_available_text_in_submission_order() {
        let mut parts = Text::default();
        let first = parts.reserve().unwrap();
        let second = parts.reserve().unwrap();
        assert!(!parts.finished());
        parts.complete(second, text("second")).unwrap();
        let partial = parts.join().unwrap();
        assert!(
            partial.as_str() == "second",
            "later partial was delayed or changed"
        );
        parts.complete(first, text("first")).unwrap();
        let final_text = parts.join().unwrap();
        assert!(
            final_text.as_str() == "first second",
            "final ordering changed"
        );
        assert!(parts.finished());
    }

    #[test]
    fn spacing_and_empty_slots_are_preserved_until_the_complete_join_is_trimmed() {
        let mut parts = Text::default();
        let first = parts.reserve().unwrap();
        let empty = parts.reserve().unwrap();
        let last = parts.reserve().unwrap();
        parts.complete(first, text("  first ")).unwrap();
        parts.complete(empty, text("")).unwrap();
        parts.complete(last, text(" second  ")).unwrap();
        let joined = parts.join().unwrap();
        assert!(
            joined.as_str() == "first    second",
            "segment spacing changed"
        );
    }

    #[test]
    fn javascript_trim_removes_bom_but_keeps_nel() {
        for edge in ['\u{feff}', '\u{00a0}', '\u{2028}'] {
            let mut parts = Text::default();
            let index = parts.reserve().unwrap();
            parts
                .complete(index, text(&format!("{edge}a{edge}")))
                .unwrap();
            let joined = parts.join().unwrap();
            assert!(joined.as_str() == "a", "JS whitespace was kept");
        }
        let mut parts = Text::default();
        let index = parts.reserve().unwrap();
        let input = "\u{0085}a\u{0085}";
        parts.complete(index, text(input)).unwrap();
        let joined = parts.join().unwrap();
        assert!(joined.as_str() == input, "NEL was incorrectly trimmed");
    }

    #[test]
    fn pending_budget_counts_unfolded_slots_and_empty_completion_settles_one() {
        use super::super::budget::compute;
        use std::time::Duration;
        use vessel_proto::dictation::ChunkSeq;
        let mut parts = Text::default();
        let first = parts.reserve().unwrap();
        let second = parts.reserve().unwrap();
        let position = ChunkSeq::new(0);
        let before = compute(parts.pending(), Duration::ZERO, Some(position), position).unwrap();
        parts.complete(second, text("")).unwrap();
        let after_one = compute(parts.pending(), Duration::ZERO, Some(position), position).unwrap();
        parts.complete(first, text("first")).unwrap();
        let after_both =
            compute(parts.pending(), Duration::ZERO, Some(position), position).unwrap();
        assert!(before.get() > after_one.get());
        assert!(after_one.get() > after_both.get());
        assert_eq!(
            before.get() - after_one.get(),
            after_one.get() - after_both.get()
        );
        assert!(parts.finished());
        assert!(parts.join().unwrap().as_str() == "first");
    }

    #[test]
    fn aggregate_text_and_separators_are_bounded_before_join_allocation() {
        for glyph in ["x", "界"] {
            let prefix = glyph.repeat((Transcript::MAX_BYTES - 1).div_euclid(glyph.len()));
            let prefix_bytes = prefix.len();
            let mut parts = Text::default();
            let full = parts.reserve().unwrap();
            let tail = parts.reserve().unwrap();
            parts.complete(full, text(&prefix)).unwrap();
            assert!(matches!(
                parts.complete(tail, text(glyph)),
                Err(TextError::TooLong)
            ));
            assert!(!parts.finished());
            parts.complete(tail, text("")).unwrap();
            let exact = parts.join().unwrap();
            assert_eq!(exact.as_str().len(), prefix_bytes);
            assert!(matches!(
                parts.complete(tail, text("")),
                Err(TextError::Commit)
            ));
            assert!(matches!(
                parts.complete(CommitIndex(usize::MAX), text("")),
                Err(TextError::Commit)
            ));
        }
    }

    #[test]
    fn empty_pending_slots_cannot_bypass_the_separator_storage_bound() {
        let mut parts = Text::default();
        for _ in 0..=Transcript::MAX_BYTES {
            let _index = parts.reserve().unwrap();
        }
        assert!(matches!(parts.reserve(), Err(TextError::TooLong)));
        assert_eq!(parts.slots.len(), Transcript::MAX_BYTES + 1);
    }
}
