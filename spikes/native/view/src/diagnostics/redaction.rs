//! Defense in depth for client-local metadata. Remote reports use an allowlist.
//! This scanner handles quoted assignments and bearer authorization without
//! retaining the second word of a credential, unlike whitespace-only masking.

const SCHEMES: [&str; 7] = [
    "ws://",
    "wss://",
    "http://",
    "https://",
    "vessel://",
    "paseo://",
    "ssh://",
];
const SECRET_KEYS: [&str; 6] = [
    "token",
    "secret",
    "password",
    "key",
    "authorization",
    "credential",
];
const MASK: &str = "[redacted]";

#[derive(Clone, Copy)]
enum ValueKind {
    Secret,
    Authorization,
}

/// Strip address/link tokens and credential assignments before display/copy.
#[must_use]
pub fn redact(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;
    let mut previous_key = false;
    while let Some(ch) = rest.chars().next() {
        if let Some(prefix) = address_prefix(rest) {
            if prefix == "x25519:" {
                result.push_str(prefix);
            }
            result.push_str(MASK);
            rest = rest.trim_start_matches(|c: char| !c.is_whitespace());
            continue;
        }
        if !previous_key && let Some((head, value, kind)) = assignment(rest) {
            result.push_str(head);
            let (quote, tail) = match value.chars().next() {
                Some(quote @ ('\'' | '"')) => (
                    Some(quote),
                    value.get(quote.len_utf8()..).unwrap_or_default(),
                ),
                Some(_) | None => (None, value),
            };
            if let Some(quote) = quote {
                result.push(quote);
            }
            result.push_str(MASK);
            rest = skip_value(tail, quote, kind);
            previous_key = false;
            continue;
        }
        result.push(ch);
        previous_key = key_char(ch);
        rest = rest.get(ch.len_utf8()..).unwrap_or_default();
    }
    result
}
fn address_prefix(text: &str) -> Option<&'static str> {
    SCHEMES.iter().copied().chain(["x25519:"]).find(|prefix| {
        text.get(..prefix.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
    })
}
fn key_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-')
}
fn assignment(text: &str) -> Option<(&str, &str, ValueKind)> {
    let end = text.find(|c: char| !key_char(c)).unwrap_or(text.len());
    let name = text.get(..end)?;
    if name.is_empty() {
        return None;
    }
    let lower = name.to_ascii_lowercase();
    if !SECRET_KEYS.iter().any(|key| lower.contains(key)) {
        return None;
    }
    let mut value = text.get(end..)?;
    // Quoted JSON keys retain the closing quote in the admitted head.
    value = value.strip_prefix(['\'', '"']).unwrap_or(value);
    value = value.trim_start_matches([' ', '\t']);
    value = value
        .strip_prefix([':', '='])?
        .trim_start_matches([' ', '\t']);
    let kept = text.len().checked_sub(value.len())?;
    Some((
        text.get(..kept)?,
        value,
        if lower.contains("authorization") {
            ValueKind::Authorization
        } else {
            ValueKind::Secret
        },
    ))
}
fn skip_value(text: &str, quote: Option<char>, kind: ValueKind) -> &str {
    let mut escaped = false;
    for (at, ch) in text.char_indices() {
        if let Some(quote) = quote {
            if !escaped && ch == quote {
                return text.get(at..).unwrap_or_default();
            }
            escaped = !escaped && ch == '\\';
        } else if ch == '\n'
            || (matches!(kind, ValueKind::Secret)
                && (ch.is_whitespace() || matches!(ch, ',' | '}' | ']')))
        {
            return text.get(at..).unwrap_or_default();
        }
    }
    ""
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quoted_json_assignments_tabs_and_bearer_tails_are_removed() {
        for (source, expected) in [
            (
                "Authorization: Bearer PRIVATE two",
                "Authorization: [redacted]",
            ),
            (
                "api_key = 'PRIVATE multi word' status",
                "api_key = '[redacted]' status",
            ),
            (
                r#"{"token":"PRIVATE multi word", "status":"ok"}"#,
                r#"{"token":"[redacted]", "status":"ok"}"#,
            ),
            ("token:\tPRIVATE\nhealthy", "token:\t[redacted]\nhealthy"),
            ("open PASEO://pair/PRIVATE", "open [redacted]"),
            (
                "한국어 password=PRIVATE 다음",
                "한국어 password=[redacted] 다음",
            ),
        ] {
            assert_eq!(redact(source), expected);
        }
    }
}
