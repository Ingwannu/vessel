//! Match Paseo's JavaScript `String.trim` without changing retained editor text.
//! ECMAScript trims `WhiteSpace` + `LineTerminator`, including `FEFF` but excluding
//! NEL (`0085`). Rust's Unicode `White_Space` predicate differs at those boundaries.
//! Source: TC39 ECMA-262 sections 12.2, 12.3 and 22.1.3.32.1.

pub(super) fn answer_text(text: &str) -> &str {
    text.trim_matches(|character| {
        matches!(character,
            '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' |
            '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' |
            '\u{205f}' | '\u{3000}' | '\u{feff}')
    })
}

#[cfg(test)]
mod tests {
    use super::answer_text;
    #[test]
    fn javascript_whitespace_does_not_erase_internal_text_or_nel() {
        assert_eq!(answer_text("\u{feff}\t a b\n\u{2003}"), "a b");
        assert_eq!(answer_text("\u{feff}\u{2028}\u{3000}"), "");
        assert_eq!(answer_text("\u{0085}"), "\u{0085}");
        assert_eq!(answer_text("\u{200b}"), "\u{200b}");
    }
}
