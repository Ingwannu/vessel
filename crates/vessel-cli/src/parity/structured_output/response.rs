//! Recover a bounded JSON response without a dynamic value tree.
//! `RawValue`'s streaming decoder handles strings/nesting; candidate scans are bounded.
use super::schema::MAX_RESPONSE_BYTES;
use vessel_proto::RawJson;
const MAX_SNIPPET_CANDIDATES: usize = 128;
#[derive(Debug, PartialEq, Eq)]
pub(super) enum DecodeError {
    TooLarge,
    Invalid,
}
pub(super) fn decode(text: &str) -> Result<RawJson, DecodeError> {
    if text.len() > MAX_RESPONSE_BYTES {
        return Err(DecodeError::TooLarge);
    }
    // A fenced answer is authoritative, matching Paseo's first-fence preference.
    if let Some(fenced) = fence(text) {
        return RawJson::from_text(fenced.trim()).map_err(|_invalid| DecodeError::Invalid);
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
    RawJson::from_text(text.trim()).map_err(|_invalid| DecodeError::Invalid)
}
fn fence(text: &str) -> Option<&str> {
    let (_, after) = text.split_once("```")?;
    let after = after.strip_prefix("json").unwrap_or(after);
    let (language, content) = after.split_once('\n')?;
    if !language.chars().all(char::is_whitespace) {
        return None;
    }
    let (content, _) = content.split_once("```")?;
    Some(content)
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
}
