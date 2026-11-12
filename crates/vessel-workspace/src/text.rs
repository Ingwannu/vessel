//! Text or not, and a page of text: shared by `file.read` and `git.show`.
//!
//! A file is text when its first 8 KiB hold no NUL byte and the bytes are
//! UTF-8 (the references' test: agent-orchestrator and GOAT use the NUL
//! byte, Paseo adds UTF-8 validity). Pages are cut back to a character
//! boundary so no page ends inside a character.

use vessel_proto::workspace::FileContent;

/// How much of the start is searched for NUL.
const SNIFF: usize = 8 * 1024;

/// Whether `bytes` read as text.
pub(crate) fn is_text(bytes: &[u8]) -> bool {
    let start = bytes.get(..SNIFF.min(bytes.len())).unwrap_or_default();
    !start.contains(&0) && std::str::from_utf8(bytes).is_ok()
}

/// A page of a file: `bytes` read from `offset`, of a file of `size` bytes,
/// whose first bytes were `start` (for the NUL test). A page that ends
/// inside a character is cut back to the character's start; `next` says
/// where the rest begins.
pub(crate) fn page(start: &[u8], bytes: &[u8], offset: u64, size: u64) -> FileContent {
    let sniff = start.get(..SNIFF.min(start.len())).unwrap_or_default();
    if sniff.contains(&0) {
        return FileContent::Binary;
    }
    let (text, used) = match std::str::from_utf8(bytes) {
        Ok(text) => (text, bytes.len()),
        // Only an incomplete character at the very end is cut; anything
        // else not UTF-8 is binary.
        Err(error) if error.error_len().is_none() => {
            let valid = error.valid_up_to();
            match std::str::from_utf8(bytes.get(..valid).unwrap_or_default()) {
                Ok(text) => (text, valid),
                Err(_never) => return FileContent::Binary,
            }
        }
        Err(_) => return FileContent::Binary,
    };
    let end = offset.saturating_add(u64::try_from(used).unwrap_or(u64::MAX));
    FileContent::Text {
        text: text.to_owned(),
        next: (end < size).then_some(end),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nul_or_invalid_utf8_is_binary_and_pages_end_on_characters() {
        assert!(is_text(b"plain\n"));
        assert!(!is_text(b"a\0b"));
        assert!(!is_text(&[0xff, 0xfe]));
        let text = "가나다".as_bytes();
        // A page that stops in the middle of `나`.
        let FileContent::Text {
            text: page_text,
            next,
        } = page(text, &text[..4], 0, 9)
        else {
            panic!("text");
        };
        assert_eq!(page_text, "가");
        assert_eq!(next, Some(3));
        assert_eq!(page(b"a\0", b"a\0", 0, 2), FileContent::Binary);
        assert_eq!(
            page(b"ab", b"ab", 0, 2),
            FileContent::Text {
                text: "ab".to_owned(),
                next: None
            }
        );
        assert_eq!(page(b"x", &[b'a', 0xff, b'b'], 0, 3), FileContent::Binary);
    }
}
