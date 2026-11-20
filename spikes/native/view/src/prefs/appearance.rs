//! How the app looks: its theme, its text sizes, the two font families,
//! the conversation's width and display.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::family::FontFamily;
use super::lenient;

/// How the app looks.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    /// Light, dark, or the system's.
    #[serde(deserialize_with = "lenient")]
    pub theme: Theme,
    /// The interface's text size.
    #[serde(deserialize_with = "lenient")]
    pub interface_size: InterfaceSize,
    /// Code's text size.
    #[serde(deserialize_with = "lenient")]
    pub code_size: CodeSize,
    /// Chat and Markdown text, in points. An old file without it is 15.
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "ContentSize::is_default"
    )]
    pub content_size: ContentSize,
    /// The interface family. Empty, or absent, means the embedded faces.
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "FontFamily::is_embedded"
    )]
    pub interface_family: FontFamily,
    /// The code family. Empty, or absent, means the embedded faces.
    #[serde(
        default,
        deserialize_with = "lenient",
        skip_serializing_if = "FontFamily::is_embedded"
    )]
    pub code_family: FontFamily,
    /// How wide the conversation is.
    #[serde(deserialize_with = "lenient")]
    pub content_width: ContentWidth,
    /// How a conversation shows tool calls.
    #[serde(deserialize_with = "lenient")]
    pub conversation: ConversationDisplay,
}

/// The app's colours.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// Whatever the system prefers, live.
    #[default]
    System,
    /// Light.
    Light,
    /// Dark.
    Dark,
    /// Paseo's zinc tint. It does not follow the system.
    Zinc,
    /// Paseo's midnight tint. It does not follow the system.
    Midnight,
    /// Paseo's Claude tint. It does not follow the system.
    Claude,
    /// Paseo's Ghostty tint. It does not follow the system.
    Ghostty,
    /// Paseo's pure black. It does not follow the system.
    PureBlack,
}

impl Theme {
    /// Every theme, in Paseo's `THEME_OPTIONS` order, which is the order
    /// Settings offers them.
    pub const ALL: [Self; 8] = [
        Self::Light,
        Self::Dark,
        Self::System,
        Self::Zinc,
        Self::Midnight,
        Self::Claude,
        Self::Ghostty,
        Self::PureBlack,
    ];
}

macro_rules! text_size {
    ($(#[$meta:meta])* $name:ident, $min:literal ..= $max:literal, $default:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "u8", into = "u8")]
        pub struct $name(u8);

        impl $name {
            /// The smallest, in points.
            pub const MIN: u8 = $min;
            /// The largest, in points.
            pub const MAX: u8 = $max;

            /// The size, in points.
            #[must_use]
            pub const fn get(self) -> u8 {
                self.0
            }

            /// Every size, smallest first.
            #[must_use]
            pub fn all() -> Vec<Self> {
                (Self::MIN..=Self::MAX).map(Self).collect()
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self($default)
            }
        }

        impl TryFrom<u8> for $name {
            type Error = OutOfRange;

            fn try_from(points: u8) -> Result<Self, OutOfRange> {
                if (Self::MIN..=Self::MAX).contains(&points) {
                    Ok(Self(points))
                } else {
                    Err(OutOfRange { min: Self::MIN, max: Self::MAX })
                }
            }
        }

        impl From<$name> for u8 {
            fn from(size: $name) -> Self {
                size.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

text_size! {
    /// The interface's text size, 12 to 20 points (14 unless chosen).
    InterfaceSize, 12 ..= 20, 14
}

text_size! {
    /// Code's text size, 10 to 18 points (12 unless chosen).
    CodeSize, 10 ..= 18, 12
}

text_size! {
    /// Chat and Markdown text, 10 to 21 points (15 unless chosen). It is
    /// an absolute size: the interface size does not scale it.
    ContentSize, 10 ..= 21, 15
}

impl ContentSize {
    /// Whether this is the desktop default, so an old file can omit it.
    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// A size outside its range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutOfRange {
    /// The smallest.
    pub min: u8,
    /// The largest.
    pub max: u8,
}

impl fmt::Display for OutOfRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a size is {} to {}", self.min, self.max)
    }
}

/// How wide the conversation is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentWidth {
    /// 768 points.
    #[default]
    Comfortable,
    /// 1152 points.
    Wide,
    /// The window's width.
    Full,
}

impl ContentWidth {
    /// Every width, narrowest first.
    pub const ALL: [Self; 3] = [Self::Comfortable, Self::Wide, Self::Full];

    /// Its width in points; `None` for the window's.
    #[must_use]
    pub const fn points(self) -> Option<u16> {
        match self {
            Self::Comfortable => Some(768),
            Self::Wide => Some(1152),
            Self::Full => None,
        }
    }
}

/// How a conversation shows tool calls.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationDisplay {
    /// Every tool call open.
    #[default]
    Full,
    /// Tool calls folded to a line.
    CollapseTools,
}

impl ConversationDisplay {
    /// Both, in the order Settings offers them.
    pub const ALL: [Self; 2] = [Self::Full, Self::CollapseTools];
}
