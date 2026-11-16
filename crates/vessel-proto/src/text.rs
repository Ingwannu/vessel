//! Shared text semantics where parity requires ECMAScript rather than Rust trimming.

/// Trim the ECMAScript `WhiteSpace` and `LineTerminator` set at both ends.
/// Interior text is unchanged. FEFF is whitespace; NEL (0085) is content.
/// This is the TC39 `TrimString` rule used by Paseo's question and project inputs:
/// <https://tc39.es/ecma262/multipage/text-processing.html#sec-trimstring>.
#[must_use]
pub fn trim_ecmascript(text: &str) -> &str {
    text.trim_matches(ecmascript_space)
}

/// The same set also delimits root command names before their typed parser.
pub(crate) fn ecmascript_space(character: char) -> bool {
    matches!(character,
        '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}' |
        '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' |
        '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

#[cfg(test)]
mod tests {
    use super::trim_ecmascript;
    #[test]
    fn every_specified_whitespace_is_trimmed_but_other_unicode_content_is_preserved() {
        let specified = "\u{0009}\u{000a}\u{000b}\u{000c}\u{000d}\u{0020}\u{00a0}\u{1680}\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}";
        assert_eq!(trim_ecmascript(specified), "");
        for edge in specified.chars() {
            assert_eq!(trim_ecmascript(&format!("{edge}answer{edge}")), "answer");
            assert_eq!(trim_ecmascript(&format!("a{edge}b")), format!("a{edge}b"));
        }
        for content in ['\u{0085}', '\u{180e}', '\u{200b}', '\u{001c}'] {
            let text = content.to_string();
            assert_eq!(trim_ecmascript(&text), text);
        }
    }
}
