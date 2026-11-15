//! One bounded readonly source and compact byte offsets, not a text engine.
//! Visible rows borrow lines; only the existing UI's visible text is copied.
use super::MAX_SOURCE_BYTES;
use std::sync::Arc;
use vessel_proto::workspace::FileSize;

/// A source that fits the native display limit; every offset is a UTF-8 edge.
#[derive(Debug)]
pub struct SourceText {
    text: Arc<str>,
    starts: Vec<u32>,
}
/// Refuse indexing beyond the same native source budget as acquisition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("source exceeds {MAX_SOURCE_BYTES} bytes")]
pub struct SourceTooLarge;
impl SourceText {
    /// Build byte starts once; no per-line string allocations or duplicate source.
    ///
    /// # Errors
    /// Refuses text beyond the native source ceiling or representable offsets.
    pub fn new(text: Arc<str>) -> Result<Self, SourceTooLarge> {
        if u64::try_from(text.len()).map_err(|_large| SourceTooLarge)? > MAX_SOURCE_BYTES {
            return Err(SourceTooLarge);
        }
        let mut starts = Vec::new();
        let mut at = 0_usize;
        for part in text.split_inclusive('\n') {
            starts.push(u32::try_from(at).map_err(|_large| SourceTooLarge)?);
            at = at.checked_add(part.len()).ok_or(SourceTooLarge)?;
        }
        Ok(Self { text, starts })
    }
    /// Shared full source for the existing Markdown worker.
    #[must_use]
    pub fn shared(&self) -> Arc<str> {
        Arc::clone(&self.text)
    }
    /// Logical source payload; row indices/preview allocations are separate.
    #[must_use]
    pub fn size(&self) -> FileSize {
        FileSize(u64::try_from(self.text.len()).unwrap_or(u64::MAX))
    }
    /// Same line count as `str::lines`, including its trailing-newline behavior.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.starts.len()
    }
    /// Borrow one line with LF/CRLF removed, preserving standalone CR.
    #[must_use]
    pub fn line(&self, index: usize) -> Option<&str> {
        let start = usize::try_from(*self.starts.get(index)?).ok()?;
        let end = self
            .starts
            .get(index.saturating_add(1))
            .and_then(|next| usize::try_from(*next).ok())
            .unwrap_or(self.text.len());
        let part = self.text.get(start..end)?;
        Some(
            part.strip_suffix('\n')
                .map_or(part, |line| line.strip_suffix('\r').unwrap_or(line)),
        )
    }
    /// Borrow only a bounded selected row range, without walking preceding rows.
    pub fn selected_lines(
        &self,
        lines: std::ops::RangeInclusive<usize>,
    ) -> impl Iterator<Item = &str> {
        let start = (*lines.start()).min(self.starts.len());
        let end = lines.end().saturating_add(1).min(self.starts.len());
        (start..end).filter_map(|index| self.line(index))
    }
    /// Iterate borrowed lines for selection intake and existing lexer consumers.
    pub fn lines(&self) -> impl Iterator<Item = &str> {
        (0..self.starts.len()).filter_map(|index| self.line(index))
    }
}
impl AsRef<str> for SourceText {
    fn as_ref(&self) -> &str {
        &self.text
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compact_rows_match_standard_lines_for_utf8_and_all_line_endings() {
        for raw in ["", "\n", "\r", "\r\n", "a\n", "a\r\nb\n\n", "가🙂\r\n나\r"] {
            let text = SourceText::new(Arc::from(raw)).unwrap();
            assert_eq!(
                text.lines().collect::<Vec<_>>(),
                raw.lines().collect::<Vec<_>>()
            );
            assert_eq!(text.line_count(), raw.lines().count());
            assert_eq!(text.line(text.line_count()), None);
            assert_eq!(text.shared().as_ref(), raw);
        }
    }
    #[test]
    fn rows_share_the_original_bytes_and_use_one_u32_per_line() {
        let original = Arc::<str>::from("first\nsecond\n");
        let text = SourceText::new(Arc::clone(&original)).unwrap();
        assert!(Arc::ptr_eq(&original, &text.shared()));
        assert_eq!(text.starts, [0, 6]);
        assert_eq!(text.line(1), Some("second"));
        assert_eq!(
            text.selected_lines(1..=usize::MAX).collect::<Vec<_>>(),
            ["second"]
        );
        let reversed = text.line_count()..=0;
        assert_eq!(text.selected_lines(reversed).next(), None);
    }
}
