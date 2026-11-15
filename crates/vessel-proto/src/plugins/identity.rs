//! Text enters a plugin contract once, through a validating constructor.

use std::{fmt, str::FromStr, sync::Arc};

use crate::key::serde_via_text;

/// Why a plugin-owned scalar cannot enter the catalog. Never echoes input:
/// even a malformed identifier can contain a credential or local path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvalidPluginValue {
    /// Empty, malformed or excessive identifiers cannot become path segments.
    #[error(
        "plugin identifiers require a lowercase letter, then lowercase letters, digits or hyphens, at most {MAX_ID_BYTES} bytes"
    )]
    Identifier,
    /// RPC names additionally accept a dot or underscore after the first letter.
    #[error(
        "plugin RPC names require a lowercase letter then lowercase letters, digits, dots, underscores or hyphens, at most {MAX_RPC_BYTES} bytes"
    )]
    RpcName,
    /// One-line captions are trimmed without discarding invalid controls.
    #[error(
        "plugin captions must be nonempty, control-free and at most {MAX_CAPTION_CHARS} characters"
    )]
    Caption,
    /// Paseo's theme parser bounds a trimmed name by this many UTF-16 units.
    #[error(
        "a theme name must be nonempty, control-free and at most {MAX_THEME_NAME_UNITS} UTF-16 units"
    )]
    ThemeName,
    /// A color's width and digits are validated together at the boundary.
    #[error("plugin colors require #rgb, #rrggbb or #rrggbbaa")]
    Color,
    /// Multiline text is bounded and may contain only newline/tab controls.
    #[error("plugin text exceeds {MAX_CONTENT_BYTES} bytes or contains unsupported controls")]
    Content,
    /// File extensions are literal suffixes, never executable glob expressions.
    #[error(
        "plugin file extensions require ASCII letters/digits or hyphens, at most {MAX_EXTENSION_BYTES} bytes, without a leading dot"
    )]
    Extension,
}

// Paseo bounds theme names in its JS schema; the identifier/RPC/caption bounds
// are Vessel's additional catalog resource limits, recorded in ADR 0079.
const MAX_ID_BYTES: usize = 128;
const MAX_RPC_BYTES: usize = 128;
const MAX_CAPTION_CHARS: usize = 120;
const MAX_THEME_NAME_UNITS: usize = 60;
const MAX_CONTENT_BYTES: usize = 16 * 1024;
const MAX_EXTENSION_BYTES: usize = 32;

macro_rules! text_type {
    ($(#[$doc:meta])* $name:ident, $validate:ident, $error:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(type = "string"))]
        pub struct $name(Arc<str>);

        impl $name {
            /// Borrow text for display or serialization without revalidation.
            #[must_use]
            pub fn as_str(&self) -> &str { &self.0 }
        }

        impl FromStr for $name {
            type Err = InvalidPluginValue;
            fn from_str(raw: &str) -> Result<Self, Self::Err> {
                $validate(raw)
                    .map(|text| Self(Arc::from(text)))
                    .ok_or(InvalidPluginValue::$error)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
        serde_via_text!($name);
    };
}

text_type!(/// An installation id; configuration may assign a different id to the same directory.
    PluginId, identifier, Identifier);
text_type!(/// A contribution's local identity, independent of its translated title.
    ContributionId, identifier, Identifier);
text_type!(/// A registered plugin method name; it never denotes a node RPC directly.
    RpcName, rpc_name, RpcName);
text_type!(/// A one-line accessible label for a contribution or control.
    Caption, caption, Caption);
text_type!(/// Paseo-compatible trimmed theme name, measured as JavaScript measures it.
    ThemeName, theme_name, ThemeName);
text_type!(/// Bounded multiline display content; core never evaluates it as code.
    ContentText, content, Content);
text_type!(/// Literal extension matcher without a dot or glob; matching ignores ASCII case.
    FileExtension, extension, Extension);

fn identifier(raw: &str) -> Option<&str> {
    (raw.len() <= MAX_ID_BYTES
        && raw.bytes().next().is_some_and(|b| b.is_ascii_lowercase())
        && raw
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'))
    .then_some(raw)
}

fn rpc_name(raw: &str) -> Option<&str> {
    (raw.len() <= MAX_RPC_BYTES
        && raw.bytes().next().is_some_and(|b| b.is_ascii_lowercase())
        && raw.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
        }))
    .then_some(raw)
}

fn caption(raw: &str) -> Option<&str> {
    let text = raw.trim();
    (!text.is_empty()
        && !raw.chars().any(char::is_control)
        && text.chars().count() <= MAX_CAPTION_CHARS)
        .then_some(text)
}

fn theme_name(raw: &str) -> Option<&str> {
    let text = crate::workspace::trim_project_text(raw);
    (!text.is_empty()
        && !raw.chars().any(char::is_control)
        && text.encode_utf16().count() <= MAX_THEME_NAME_UNITS)
        .then_some(text)
}

fn content(raw: &str) -> Option<&str> {
    (raw.len() <= MAX_CONTENT_BYTES
        && !raw
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\t')))
    .then_some(raw)
}

fn extension(raw: &str) -> Option<&str> {
    (!raw.is_empty()
        && raw.len() <= MAX_EXTENSION_BYTES
        && raw.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
    .then_some(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_refuse_path_traversal_unicode_controls_and_excess() {
        for text in ["", "A", "../x", "a/b", "한글", "a\n", " a", "a_b", "a.b"] {
            assert_eq!(
                text.parse::<PluginId>(),
                Err(InvalidPluginValue::Identifier)
            );
            assert!(serde_json::from_str::<PluginId>(&format!("{text:?}")).is_err());
        }
        assert!("a".repeat(MAX_ID_BYTES).parse::<PluginId>().is_ok());
        assert!(
            "a".repeat(MAX_ID_BYTES.saturating_add(1))
                .parse::<PluginId>()
                .is_err()
        );
        assert!("a-2".parse::<ContributionId>().is_ok());
    }

    #[test]
    fn theme_name_uses_utf16_and_never_normalizes_away_controls() {
        assert!(
            "😀"
                .repeat(MAX_THEME_NAME_UNITS.div_euclid(2))
                .parse::<ThemeName>()
                .is_ok()
        );
        assert!(
            "😀"
                .repeat((MAX_THEME_NAME_UNITS.div_euclid(2)).saturating_add(1))
                .parse::<ThemeName>()
                .is_err()
        );
        assert!("ok\n".parse::<ThemeName>().is_err());
        assert_eq!(
            "  테마  ".parse::<ThemeName>().map(|name| name.to_string()),
            Ok(String::from("테마"))
        );
    }

    #[test]
    fn rpc_names_and_caption_have_distinct_grammars() {
        assert!("settings.account_read-v1".parse::<RpcName>().is_ok());
        assert!(".read".parse::<RpcName>().is_err());
        assert!("테마 선택".parse::<Caption>().is_ok());
        assert!("a\u{85}b".parse::<Caption>().is_err());
        assert!(
            "a".repeat(MAX_CAPTION_CHARS.saturating_add(1))
                .parse::<Caption>()
                .is_err()
        );
    }

    #[test]
    fn display_content_and_file_matchers_cannot_smuggle_resource_paths() {
        assert!("line one\n\tline two".parse::<ContentText>().is_ok());
        assert!("bad\0text".parse::<ContentText>().is_err());
        assert!("x".repeat(MAX_CONTENT_BYTES).parse::<ContentText>().is_ok());
        assert!(
            "x".repeat(MAX_CONTENT_BYTES.saturating_add(1))
                .parse::<ContentText>()
                .is_err()
        );
        for text in ["", ".rs", "../rs", "*.rs", "rs/js", "rs\n"] {
            assert!(text.parse::<FileExtension>().is_err());
        }
        assert!("RS".parse::<FileExtension>().is_ok());
        assert!(
            "x".repeat(MAX_EXTENSION_BYTES.saturating_add(1))
                .parse::<FileExtension>()
                .is_err()
        );
    }
    #[test]
    fn theme_names_reuse_javascript_bom_trimming_before_utf16_validation() {
        assert_eq!(
            "\u{feff}".parse::<ThemeName>(),
            Err(InvalidPluginValue::ThemeName)
        );
        let wrapped = "\u{feff} Theme \u{feff}"
            .parse::<ThemeName>()
            .expect("theme");
        assert_eq!(wrapped.as_str(), "Theme");
        assert_eq!(
            serde_json::to_string(&wrapped).expect("encode"),
            "\"Theme\""
        );
        let at_bound = format!("\u{feff}{}\u{feff}", "한".repeat(MAX_THEME_NAME_UNITS));
        assert!(at_bound.parse::<ThemeName>().is_ok());
        let over = format!(
            "\u{feff}{}\u{feff}",
            "한".repeat(MAX_THEME_NAME_UNITS.saturating_add(1))
        );
        assert!(over.parse::<ThemeName>().is_err());
    }
}
