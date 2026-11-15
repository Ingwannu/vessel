//! Decode the validated family stack once at its input edge. These are font
//! names only: neither a URL nor a filesystem path is ever opened.
use std::{iter::Peekable, str::Chars};

/// A decoded family name, including quoted commas or escaped characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaceName(String);
impl FaceName {
    /// The platform font-system lookup name, without CSS quoting.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Generic stacks map to native embedded fallbacks; they are never queried
/// as a possibly bitmap platform generic family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenericFamily {
    /// A normal interface stack.
    Sans,
    /// A code stack.
    Mono,
    /// Other generic categories use the embedded readable fallback.
    Readable,
}

/// One face in the user's ordered stack, already decoded at the edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FamilyFace {
    /// An explicit font family; availability is a native app concern.
    Named(FaceName),
    /// A CSS generic intent, never a font-file or URL request.
    Generic(GenericFamily),
}

#[derive(Clone, Copy)]
enum Quotation {
    Bare,
    Quoted,
}

#[derive(Default)]
enum Separator {
    #[default]
    Absent,
    Space,
}

fn space(name: &mut String, pending: &mut Separator) {
    if matches!(std::mem::take(pending), Separator::Space) {
        name.push(' ');
    }
}

pub(super) fn decode(text: &str) -> Vec<FamilyFace> {
    let mut chars = text.chars().peekable();
    let mut quote = None;
    let mut quoted = Quotation::Bare;
    let mut pending = Separator::Absent;
    let mut name = String::new();
    let mut faces = Vec::new();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(escaped) = escape(&mut chars) {
                space(&mut name, &mut pending);
                name.push(escaped);
            }
        } else if let Some(current) = quote {
            if ch == current {
                quote = None;
            } else {
                name.push(ch);
            }
        } else if ch == '\'' || ch == '"' {
            space(&mut name, &mut pending);
            quote = Some(ch);
            quoted = Quotation::Quoted;
        } else if ch == ',' {
            push(&mut faces, &mut name, quoted);
            quoted = Quotation::Bare;
            pending = Separator::Absent;
        } else if ch == ' ' {
            pending = if name.is_empty() {
                Separator::Absent
            } else {
                Separator::Space
            };
        } else {
            space(&mut name, &mut pending);
            name.push(ch);
        }
    }
    push(&mut faces, &mut name, quoted);
    faces
}

fn push(faces: &mut Vec<FamilyFace>, name: &mut String, quoted: Quotation) {
    let name = std::mem::take(name);
    let name = name.as_str();
    if name.is_empty() {
        return;
    }
    let generic = matches!(quoted, Quotation::Bare)
        .then(|| match name.to_ascii_lowercase().as_str() {
            "sans-serif" | "system-ui" | "ui-sans-serif" => Some(GenericFamily::Sans),
            "monospace" | "ui-monospace" => Some(GenericFamily::Mono),
            "serif" | "ui-serif" | "cursive" | "fantasy" | "emoji" | "math" | "fangsong" => {
                Some(GenericFamily::Readable)
            }
            _ => None,
        })
        .flatten();
    faces.push(match generic {
        Some(generic) => FamilyFace::Generic(generic),
        None => FamilyFace::Named(FaceName(name.to_owned())),
    });
}

fn escape(chars: &mut Peekable<Chars<'_>>) -> Option<char> {
    const MAX_HEX_DIGITS: usize = 6;
    let first = chars.next()?;
    let Some(mut point) = first.to_digit(16) else {
        return Some(first);
    };
    for _ in 1..MAX_HEX_DIGITS {
        let Some(digit) = chars.peek().and_then(|ch| ch.to_digit(16)) else {
            break;
        };
        // At most six hex digits: checked arithmetic keeps that invariant local.
        point = point.checked_mul(16)?.checked_add(digit)?;
        let _digit = chars.next();
    }
    if chars.peek() == Some(&' ') {
        let _space = chars.next();
    }
    Some(
        char::from_u32(point)
            .filter(|ch| *ch != '\0')
            .unwrap_or('\u{fffd}'),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn commas_quotes_hex_escapes_and_generic_intent_are_decoded_once() {
        let parsed =
            super::super::FontFamily::parse(r#"'A, B', \47 eist, monospace, "serif""#).unwrap();
        let faces = parsed.faces();
        assert_eq!(faces.len(), 4);
        assert!(matches!(&faces[0], FamilyFace::Named(name) if name.as_str() == "A, B"));
        assert!(matches!(&faces[1], FamilyFace::Named(name) if name.as_str() == "Geist"));
        assert_eq!(faces[2], FamilyFace::Generic(GenericFamily::Mono));
        assert!(matches!(&faces[3], FamilyFace::Named(name) if name.as_str() == "serif"));
        assert!(super::super::FontFamily::default().faces().is_empty());
    }
}
