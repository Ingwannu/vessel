//! Saved text format is independent of the editor's normalized lines.
//! Paseo detects the first line break and remembers a UTF-8 BOM, so a
//! normal edit must not silently turn CRLF/CR files into LF files.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum LineEnding {
    #[default]
    Lf,
    CrLf,
    Cr,
}

/// The original byte-level text conventions, retained beside the buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextFormat {
    bom: bool,
    ending: LineEnding,
}

impl TextFormat {
    /// No BOM or observed line break; ordinary LF for newly inserted lines.
    pub const DEFAULT: Self = Self {
        bom: false,
        ending: LineEnding::Lf,
    };

    /// Decode a complete UTF-8 source into editor text and its conventions.
    #[must_use]
    pub fn decode(source: &str) -> (Self, String) {
        let bom = source.starts_with('\u{feff}');
        let body = source.strip_prefix('\u{feff}').unwrap_or(source);
        let mut chars = body.chars().peekable();
        let mut ending = LineEnding::Lf;
        while let Some(character) = chars.next() {
            match character {
                '\n' => break,
                '\r' => {
                    ending = if chars.peek() == Some(&'\n') {
                        LineEnding::CrLf
                    } else {
                        LineEnding::Cr
                    };
                    break;
                }
                _ => {}
            }
        }
        (Self { bom, ending }, normalize_lines(body))
    }

    /// Reconstruct the bytes sent to the guarded writer.
    #[must_use]
    pub fn encode(self, normalized: &str) -> String {
        let separator = match self.ending {
            LineEnding::Lf => "\n",
            LineEnding::CrLf => "\r\n",
            LineEnding::Cr => "\r",
        };
        let mut source = if self.bom {
            "\u{feff}".to_owned()
        } else {
            String::new()
        };
        source.push_str(&normalized.replace('\n', separator));
        source
    }
}

/// Editor paste/input can carry CRLF or CR even though stored lines use LF.
#[must_use]
pub fn normalize_lines(source: &str) -> String {
    source.replace("\r\n", "\n").replace('\r', "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_bom_first_separator_and_trailing_newline() {
        for source in [
            "a\nb\n",
            "a\r\nb\r\n",
            "a\rb\r",
            "\u{feff}a\r\nb\r\n",
            "",
            "\u{feff}",
        ] {
            let (format, text) = TextFormat::decode(source);
            assert_eq!(format.encode(&text), source);
            assert!(!text.starts_with('\u{feff}'));
        }
        let (format, text) = TextFormat::decode("a\r\nb\nc\r");
        assert_eq!(text, "a\nb\nc\n");
        assert_eq!(format.encode(&text), "a\r\nb\r\nc\r\n");
    }
}
