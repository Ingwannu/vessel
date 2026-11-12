//! Credentials out of git's messages before they reach a log or a client.
//!
//! git repeats a remote's URL in its errors, and a URL can carry a user and
//! a password or a token (`https://user:token@host/…`). GOAT sends a fixed
//! reason instead of stderr for that reason; Vessel classifies what it can
//! into typed refusals and passes the rest through here.

/// The last lines of `text` (at most [`TAIL_BYTES`]), every URL's user
/// information replaced by `***`.
///
/// Redacted whole, then cut: a cut through a URL would leave the end of
/// its user information without the `://` that marks it.
pub(crate) fn tail(text: &str) -> String {
    let redacted = redact(text);
    let text = redacted.trim();
    let start = text.len().saturating_sub(TAIL_BYTES);
    let start = (start..text.len())
        .find(|index| text.is_char_boundary(*index))
        .unwrap_or(text.len());
    text.get(start..).unwrap_or_default().to_owned()
}

/// How much of a message is kept.
pub(crate) const TAIL_BYTES: usize = 2048;

/// `text` with the user information of every `scheme://user@host` replaced.
pub(crate) fn redact(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(index) = rest.find("://") {
        let (before, after) = rest.split_at(index);
        out.push_str(before);
        out.push_str("://");
        let after = after.get(3..).unwrap_or_default();
        let end = after
            .find(|c: char| c.is_whitespace() || matches!(c, '/' | '\'' | '"'))
            .unwrap_or(after.len());
        let authority = after.get(..end).unwrap_or_default();
        match authority.rfind('@') {
            Some(at) => {
                out.push_str("***");
                out.push_str(authority.get(at..).unwrap_or_default());
            }
            None => out.push_str(authority),
        }
        rest = after.get(end..).unwrap_or_default();
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_information_never_survives() {
        assert_eq!(
            redact("fatal: unable to access 'https://me:ghp_secret@github.com/o/r.git/': 403"),
            "fatal: unable to access 'https://***@github.com/o/r.git/': 403"
        );
        assert_eq!(
            redact("remote https://github.com/o/r and ssh://git@host:22/x"),
            "remote https://github.com/o/r and ssh://***@host:22/x"
        );
        assert_eq!(redact("no url here"), "no url here");
        let long = format!("{}https://a:b@h/x", "x".repeat(5000));
        let cut = tail(&long);
        assert!(cut.len() <= TAIL_BYTES + 3);
        assert!(!cut.contains("a:b"), "{cut}");
    }

    #[test]
    fn a_cut_through_a_url_leaves_none_of_its_user_information() {
        // The cut falls inside the token: the start of the URL is gone.
        let token = "t".repeat(64);
        let text = format!("https://me:{token}@host/x {}", "y".repeat(TAIL_BYTES - 40));
        let cut = tail(&text);
        assert!(cut.len() <= TAIL_BYTES, "{}", cut.len());
        assert!(!cut.contains("tttt"), "{cut}");
    }
}
