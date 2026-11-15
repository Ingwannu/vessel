//! The stored Appearance choices: palette, independent text axes and
//! safe family stacks, syntax, conversation width and display.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::lenient;
use crate::appearance::{ContentPixels, FontFamily, SyntaxTheme};

/// How the app looks.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    /// A named palette, or the system's light/dark choice.
    #[serde(deserialize_with = "lenient")]
    pub theme: Theme,
    /// The interface's text size.
    #[serde(deserialize_with = "lenient")]
    pub interface_size: InterfaceSize,
    /// Code's text size.
    #[serde(deserialize_with = "lenient")]
    pub code_size: CodeSize,
    /// Chat content's absolute size, independent of the UI scale.
    #[serde(
        deserialize_with = "lenient",
        skip_serializing_if = "ContentSize::is_default"
    )]
    pub content_size: ContentSize,
    /// Interface stack; empty follows the embedded default.
    #[serde(
        deserialize_with = "lenient",
        skip_serializing_if = "FontFamily::is_default"
    )]
    pub interface_family: FontFamily,
    /// Code and terminal stack; empty follows the embedded mono default.
    #[serde(
        deserialize_with = "lenient",
        skip_serializing_if = "FontFamily::is_default"
    )]
    pub mono_family: FontFamily,
    /// Code's colour palette, independent of the app palette.
    #[serde(
        deserialize_with = "lenient",
        skip_serializing_if = "SyntaxTheme::is_default"
    )]
    pub syntax_theme: SyntaxTheme,
    /// How wide the conversation is.
    #[serde(deserialize_with = "lenient")]
    pub content_width: ContentWidth,
    /// How a conversation shows tool calls.
    #[serde(deserialize_with = "lenient")]
    pub conversation: ConversationDisplay,
    /// Whether the app moves (Settings › Appearance › Animations).
    #[serde(deserialize_with = "lenient")]
    pub animations: Animations,
}

/// Whether the app's changes move (the owner's word 39: every change a
/// person causes has a short transition, and a setting turns them off so
/// every change is instant). Kept with the app's preferences only: the
/// page has no such setting yet (its motion follows the system's), and
/// the page's document ignores a field it does not know.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Animations {
    /// Transitions play (the default).
    #[default]
    On,
    /// Every change shows its end at once.
    Off,
}

impl Animations {
    /// Whether transitions play.
    #[must_use]
    pub const fn on(self) -> bool {
        matches!(self, Self::On)
    }
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
    /// Paseo's neutral gray palette.
    Zinc,
    /// Paseo's blue-tinted palette.
    Midnight,
    /// Paseo's warm neutral palette.
    Claude,
    /// Paseo's slate-blue palette.
    Ghostty,
    /// Paseo's black-background palette.
    PureBlack,
}

impl Theme {
    /// Every theme, in the order Settings offers them.
    pub const ALL: [Self; 8] = [
        Self::System,
        Self::Light,
        Self::Dark,
        Self::Zinc,
        Self::Midnight,
        Self::Claude,
        Self::Ghostty,
        Self::PureBlack,
    ];

    /// Paseo's shortcut cycles the catalogue, independently of the
    /// System-first picker (`styles/theme.ts`, `getNextThemePreference`).
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::System,
            Self::System => Self::Zinc,
            Self::Zinc => Self::Midnight,
            Self::Midnight => Self::Claude,
            Self::Claude => Self::Ghostty,
            Self::Ghostty => Self::PureBlack,
            Self::PureBlack => Self::Light,
        }
    }
}

#[cfg(test)]
mod override_tests;
#[cfg(test)]
mod theme_tests;

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

            /// Omit an unchanged default from legacy documents.
            #[must_use]
            pub fn is_default(&self) -> bool { *self == Self::default() }

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
    /// The interface size, bounded by `MIN`/`MAX` (desktop default 14).
    InterfaceSize, 10 ..= 21, 14
}

text_size! {
    /// Code size, bounded by `MIN`/`MAX` (default 12).
    CodeSize, 9 ..= 22, 12
}

text_size! {
    /// Chat content's independent absolute size (desktop default 15).
    ContentSize, 10 ..= 21, 15
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
    /// Follow the current default content maximum.
    #[default]
    Comfortable,
    /// 1152 points.
    Wide,
    /// The window's width, retained for existing preferences.
    Full,
    /// A validated numeric override; the default remains a following choice.
    Pixels(ContentPixels),
}

impl ContentWidth {
    /// Preserve following/legacy intent when the effective value is unchanged.
    #[must_use]
    pub fn commit(self, next: ContentPixels) -> Self {
        if self.points() == Some(next.get()) {
            self
        } else {
            Self::Pixels(next)
        }
    }

    /// Whether reset would remove an explicit or legacy override.
    #[must_use]
    pub const fn resettable(self) -> bool {
        !matches!(self, Self::Comfortable)
    }

    /// Every width, narrowest first.
    pub const ALL: [Self; 3] = [Self::Comfortable, Self::Wide, Self::Full];

    /// Its width in points; `None` for the window's.
    #[must_use]
    pub const fn points(self) -> Option<u16> {
        match self {
            Self::Comfortable => Some(ContentPixels::DEFAULT.get()),
            Self::Wide => Some(1152),
            Self::Full => None,
            Self::Pixels(value) => Some(value.get()),
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
