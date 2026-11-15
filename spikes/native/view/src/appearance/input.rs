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
