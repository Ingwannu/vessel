//! A font family a person names. Empty means the embedded faces, not a
//! generic family and not Paseo's platform stack (ADR 0070).

use std::fmt;

use serde::{Deserialize, Serialize};

/// The most characters in a family, counted as Paseo counts them
/// (`storage.ts` `MAX_FONT_FAMILY_LENGTH`: a JavaScript string's length,
/// which is UTF-16 code units).
pub const MAX_FAMILY_CHARS: usize = 200;

/// Why a named family was refused. A preference file that holds a bad
/// one still loads the embedded faces; only an explicit change returns
/// this, because Paseo's silent replacement is a drop (ADR 0070).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FamilyError {
    /// Longer than [`MAX_FAMILY_CHARS`] after trimming.
    #[error("a font family is at most {MAX_FAMILY_CHARS} characters")]
    Length,
    /// `;{}<>` or a control character would break the declaration.
    #[error("a font family cannot contain ; {{ }} < > or a control character")]
    Broken,
    /// `( ) [ ] / *` outside quotes would open a comment or a call.
    #[error("a font family cannot contain ( ) [ ] / * outside quotes")]
    Mark,
    /// A quote or a backslash was still open at the end.
    #[error("a font family has an unfinished quote or escape")]
    Unfinished,
}

/// The family a person typed, after Paseo's sanitiser
/// (`storage.ts` `sanitizeFontFamily`). Empty means the embedded faces.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct FontFamily(String);

impl FontFamily {
    /// Whether this is the embedded stack. Empty is that choice, not a
    /// missing value, so a file can record it by leaving the field out.
    #[must_use]
    pub fn is_embedded(&self) -> bool {
        self.0.is_empty()
    }

    /// The text as stored, quotes included when the person typed them.
    #[must_use]
    pub fn get(&self) -> &str {
        &self.0
    }

    /// The face to draw: the first family in the list, quotes and
    /// escapes removed. Anything after the first comma is not drawn,
    /// so a generic name in a stack is never added (ADR 0070).
    #[must_use]
    pub fn face(&self) -> String {
        let mut face = String::new();
        let mut quote = None;
        let mut escaped = false;
        for c in self.0.chars() {
            if escaped {
                face.push(c);
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if let Some(open) = quote {
                if c == open {
                    quote = None;
                } else {
                    face.push(c);
                }
            } else if c == '\'' || c == '"' {
                quote = Some(c);
            } else if c == ',' {
                break;
            } else {
                face.push(c);
            }
        }
        face.trim().to_owned()
    }
}

impl Default for FontFamily {
    fn default() -> Self {
        Self(String::new())
    }
}

impl TryFrom<String> for FontFamily {
    type Error = FamilyError;

    fn try_from(value: String) -> Result<Self, FamilyError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Ok(Self(String::new()));
        }
        if trimmed.encode_utf16().count() > MAX_FAMILY_CHARS {
            return Err(FamilyError::Length);
        }
        if trimmed.chars().any(is_broken) {
            return Err(FamilyError::Broken);
        }
        let mut quote = None;
        let mut escaped = false;
        for c in trimmed.chars() {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if let Some(open) = quote {
                if c == open {
                    quote = None;
                }
            } else if c == '\'' || c == '"' {
                quote = Some(c);
            } else if matches!(c, '(' | ')' | '[' | ']' | '/' | '*') {
                return Err(FamilyError::Mark);
            }
        }
        if quote.is_some() || escaped {
            Err(FamilyError::Unfinished)
        } else {
            Ok(Self(trimmed.to_owned()))
        }
    }
}

impl From<FontFamily> for String {
    fn from(family: FontFamily) -> Self {
        family.0
    }
}

impl fmt::Display for FontFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// `;{}<>` or a control character, which Paseo rejects anywhere in the
/// string, including inside quotes (`storage.ts`).
fn is_broken(c: char) -> bool {
    matches!(c, ';' | '{' | '}' | '<' | '>') || u32::from(c) <= 0x1f
}

#[cfg(test)]
mod tests {
    use super::{FamilyError, FontFamily};

    fn family(text: &str) -> Result<FontFamily, FamilyError> {
        FontFamily::try_from(text.to_owned())
    }

    #[test]
    fn empty_means_the_embedded_faces() {
        let blank = family("  ").unwrap();
        assert!(blank.is_embedded());
        assert_eq!(blank.face(), "");
    }

    #[test]
    fn a_stack_keeps_its_text_and_draws_the_first_face() {
        let stored = family("'Fira Code', monospace").unwrap();
        assert_eq!(stored.get(), "'Fira Code', monospace");
        assert_eq!(stored.face(), "Fira Code");
        assert_eq!(
            family("\"Fira Code\", monospace").unwrap().face(),
            "Fira Code"
        );
        assert_eq!(family("'Font\\'s name'").unwrap().face(), "Font's name");
        assert_eq!(family("Font\\ Name").unwrap().face(), "Font Name");
        assert_eq!(family("'Font (Mono)'").unwrap().face(), "Font (Mono)");
        assert_eq!(family("Menlo").unwrap().face(), "Menlo");
    }

    #[test]
    fn a_broken_declaration_is_refused() {
        assert_eq!(family("foo/*"), Err(FamilyError::Mark));
        assert_eq!(family("foo("), Err(FamilyError::Mark));
        assert_eq!(family("foo["), Err(FamilyError::Mark));
        assert_eq!(family("'foo"), Err(FamilyError::Unfinished));
        assert_eq!(family("\"foo"), Err(FamilyError::Unfinished));
        assert_eq!(family("Menlo, 'foo"), Err(FamilyError::Unfinished));
        assert_eq!(family("foo\\"), Err(FamilyError::Unfinished));
        assert_eq!(family("a;b"), Err(FamilyError::Broken));
        assert_eq!(family("a\nb"), Err(FamilyError::Broken));
        let long = "a".repeat(201);
        assert_eq!(family(&long), Err(FamilyError::Length));
        assert!(family(&"a".repeat(200)).is_ok());
    }
}
