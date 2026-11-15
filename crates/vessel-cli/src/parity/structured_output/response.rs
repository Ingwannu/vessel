//! Recover a bounded JSON response without a dynamic value tree.
//! `RawValue`'s streaming decoder handles strings/nesting; candidate scans are bounded.
use super::schema::MAX_RESPONSE_BYTES;
use vessel_proto::RawJson;
const MAX_SNIPPET_CANDIDATES: usize = 128;
const MAX_FENCE_CANDIDATES: usize = 128;
#[derive(Debug, PartialEq, Eq)]
pub(super) enum DecodeError {
    TooLarge,
    FenceBudget,
    Invalid,
}
pub(super) fn decode(text: &str) -> Result<RawJson, DecodeError> {
    if text.len() > MAX_RESPONSE_BYTES {
        return Err(DecodeError::TooLarge);
    }
    // A fenced answer is authoritative, matching Paseo's first-fence preference.
    if let Some(fenced) = fence(text)? {
        return RawJson::from_text(fenced.trim_matches(ecma_whitespace))
            .map_err(|_invalid| DecodeError::Invalid);
    }
    for (offset, _) in text
        .char_indices()
        .filter(|(_, c)| matches!(c, '{' | '['))
        .take(MAX_SNIPPET_CANDIDATES)
    {
        let Some(tail) = text.get(offset..) else {
            continue;
        };
        let mut stream = serde_json::Deserializer::from_str(tail)
            .into_iter::<Box<serde_json::value::RawValue>>();
        if let Some(Ok(candidate)) = stream.next() {
            return RawJson::from_text(candidate.get()).map_err(|_invalid| DecodeError::Invalid);
        }
    }
    RawJson::from_text(text.trim_matches(ecma_whitespace)).map_err(|_invalid| DecodeError::Invalid)
}
fn fence(text: &str) -> Result<Option<&str>, DecodeError> {
    let mut candidates = 0_usize;
    // Examine overlapping triples: four opening backticks can match at the second.
    for (offset, _) in text.match_indices('`') {
        let Some(after) = text.get(offset..).and_then(|tail| tail.strip_prefix("```")) else {
            continue;
        };
        candidates = candidates.checked_add(1).ok_or(DecodeError::FenceBudget)?;
        if candidates > MAX_FENCE_CANDIDATES {
            return Err(DecodeError::FenceBudget);
        }
        let after = after.strip_prefix("json").unwrap_or(after);
        let whitespace_end = after.find(|c| !ecma_whitespace(c)).unwrap_or(after.len());
        let Some(last_closing) = after.rfind("\n```") else {
            continue;
        };
        // Reproduce greedy \s* backtracking: choose its last newline that
        // still leaves a later closing fence, then take the first such close.
        let Some(opening_line) = after
            .get(..whitespace_end.min(last_closing))
            .and_then(|prefix| prefix.rfind('\n'))
        else {
            continue;
        };
        let Some(content) = after.get(opening_line.saturating_add(1)..) else {
            continue;
        };
        // A closing fence needs its own newline, independently of opening whitespace.
        if let Some((content, _)) = content.split_once("\n```") {
            return Ok(Some(content));
        }
    }
    Ok(None)
}
fn ecma_whitespace(c: char) -> bool {
    // JavaScript \s/trim includes BOM and excludes NEL, unlike Rust whitespace.
    matches!(c, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}'
        | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}'
        | '\u{205f}' | '\u{3000}' | '\u{feff}')
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn response_recovers_fences_and_nested_quoted_delimiters_without_a_tree() {
        for (input, expected) in [
            (
                "Prose: {\"text\":\"}\",\"items\":[{\"n\":1}]} trailing",
                r#"{"text":"}","items":[{"n":1}]}"#,
            ),
            ("```json\n{\"ok\":true}\n```", r#"{"ok":true}"#),
            ("true", "true"),
        ] {
            assert_eq!(decode(input).unwrap().as_text(), expected);
        }
        assert_eq!(
            decode("```json\ninvalid\n``` {\"ok\":true}").unwrap_err(),
            DecodeError::Invalid
        );
        assert_eq!(
            decode(&"x".repeat(MAX_RESPONSE_BYTES.saturating_add(1))).unwrap_err(),
            DecodeError::TooLarge
        );
    }
    #[test]
    fn fence_selection_matches_the_reference_regex_before_json_snippet_fallback() {
        for input in [
            "```json\ninvalid``` {\"ok\":true}",
            "````json\n{\"ok\":true}\n```",
            "```\u{0085}\ninvalid\n``` {\"ok\":true}",
        ] {
            assert_eq!(decode(input).unwrap().as_text(), r#"{"ok":true}"#);
        }
        // The reference also treats a previous language fence's closing ticks
        // as an opening match; it must not silently fall back to an earlier object.
        let input = "```text\n{\"wrong\":1}\n```\n```json\n{\"right\":2}\n```";
        assert_eq!(decode(input).unwrap_err(), DecodeError::Invalid);
        let greedy = "````jsonJSON```\n\r\n````\u{feff}JSON\r\n````\r\n";
        assert_eq!(
            fence(greedy)
                .unwrap()
                .map(|text| text.trim_matches(ecma_whitespace)),
            Some("````\u{feff}JSON")
        );
        assert_eq!(decode("\u{feff}true\u{feff}").unwrap().as_text(), "true");
        let excessive = "```x".repeat(MAX_FENCE_CANDIDATES.saturating_add(1));
        assert_eq!(decode(&excessive).unwrap_err(), DecodeError::FenceBudget);
    }
}
