//! Lexical whitespace at the Appearance preference input boundary.
//!
//! JavaScript String.trim includes the BOM and excludes NEL; Rust's general
//! Unicode trim is different. Both family and numeric drafts use this rule.

pub(super) fn trim(text: &str) -> &str {
    text.trim_matches(|ch| {
        matches!(ch,
            '\u{9}'..='\u{d}' | '\u{20}' | '\u{a0}' | '\u{1680}' |
            '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' |
            '\u{205f}' | '\u{3000}' | '\u{feff}'
        )
    })
}

/// Raw text at the editing edge, never a committed preference or a second
/// editor state. The existing text engine remains its sole live owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftText(String);
impl DraftText {
    /// Construct only at the sibling form's raw-input boundary.
    pub(super) fn new(text: String) -> Self {
        Self(text)
    }
    /// The input data, for the caller's existing editor to retain/replace.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
