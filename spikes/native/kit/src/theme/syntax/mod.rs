//! Semantic syntax colours, independent of app palettes and renderers.
//! Source: Paseo packages/highlight/src/{themes,colors}.ts (0.15.4).
//! One role expansion retains all twenty styles. Dracula is dark-only;
//! Nord follows the actual paired resolver, despite its stale file comment.

mod palettes;
mod selection;
pub(crate) use selection::select;
pub use selection::selected;

use crate::Scheme;
use gpui::Hsla;

/// A syntax palette, separate from the view preference and app colour palette.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SyntaxPalette {
    /// Github syntax colours.
    Github,
    /// Catppuccin syntax colours.
    Catppuccin,
    /// Dracula syntax colours.
    Dracula,
    /// `TokyoNight` syntax colours.
    TokyoNight,
    /// One syntax colours.
    #[default]
    One,
    /// Nord syntax colours.
    Nord,
    /// Gruvbox syntax colours.
    Gruvbox,
    /// Solarized syntax colours.
    Solarized,
}

/// A semantic token produced by a lexer, never a renderer-specific colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxStyle {
    /// Keyword token.
    Keyword,
    /// Comment token.
    Comment,
    /// String token.
    String,
    /// Number token.
    Number,
    /// A literal/constant token; retains the editor's existing name.
    Constant,
    /// Function token.
    Function,
    /// Definition token.
    Definition,
    /// Class token.
    Class,
    /// Type token.
    Type,
    /// Tag token.
    Tag,
    /// Attribute token.
    Attribute,
    /// Property token.
    Property,
    /// Variable token.
    Variable,
    /// Operator token.
    Operator,
    /// Punctuation token.
    Punctuation,
    /// Regexp token.
    Regexp,
    /// Escape token.
    Escape,
    /// Meta token.
    Meta,
    /// Heading token.
    Heading,
    /// Link token.
    Link,
}

impl SyntaxStyle {
    /// The existing editor's external style key, shared by readonly rows.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Keyword => "keyword",
            Self::Comment => "comment",
            Self::String => "string",
            Self::Number => "number",
            Self::Constant => "constant",
            Self::Function => "function",
            Self::Definition => "definition",
            Self::Class => "class",
            Self::Type => "type",
            Self::Tag => "tag",
            Self::Attribute => "attribute",
            Self::Property => "property",
            Self::Variable => "variable",
            Self::Operator => "operator",
            Self::Punctuation => "punctuation",
            Self::Regexp => "regexp",
            Self::Escape => "escape",
            Self::Meta => "meta",
            Self::Heading => "heading",
            Self::Link => "link",
        }
    }

    /// Decode the external editor highlighter's string vocabulary at its edge.
    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name {
            "keyword" => Some(Self::Keyword),
            "comment" => Some(Self::Comment),
            "string" => Some(Self::String),
            "number" => Some(Self::Number),
            "literal" | "constant" => Some(Self::Constant),
            "function" => Some(Self::Function),
            "definition" => Some(Self::Definition),
            "class" => Some(Self::Class),
            "type" => Some(Self::Type),
            "tag" => Some(Self::Tag),
            "attribute" => Some(Self::Attribute),
            "property" => Some(Self::Property),
            "variable" => Some(Self::Variable),
            "operator" => Some(Self::Operator),
            "punctuation" => Some(Self::Punctuation),
            "regexp" => Some(Self::Regexp),
            "escape" => Some(Self::Escape),
            "meta" => Some(Self::Meta),
            "heading" => Some(Self::Heading),
            "link" => Some(Self::Link),
            _ => None,
        }
    }

    /// Every source role, for complete consumers and palette audits.
    pub const ALL: [Self; 20] = [
        Self::Keyword,
        Self::Comment,
        Self::String,
        Self::Number,
        Self::Constant,
        Self::Function,
        Self::Definition,
        Self::Class,
        Self::Type,
        Self::Tag,
        Self::Attribute,
        Self::Property,
        Self::Variable,
        Self::Operator,
        Self::Punctuation,
        Self::Regexp,
        Self::Escape,
        Self::Meta,
        Self::Heading,
        Self::Link,
    ];
}

/// One caller-classified UTF-8 byte run for an existing text renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeRun {
    /// Byte positions within its supplied row text.
    pub range: std::ops::Range<usize>,
    /// Semantic colour class, never a private colour table.
    pub style: SyntaxStyle,
}

/// Complete source colours. A fixed tuple keeps lookup exhaustive without
/// a panicking numeric index or a second token expansion per theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntaxColors([u32; 20]);

impl SyntaxColors {
    /// The authored opaque sRGB value; useful for exact adapter comparisons.
    #[must_use]
    pub fn rgb(self, style: SyntaxStyle) -> u32 {
        let [
            keyword,
            comment,
            string,
            number,
            literal,
            function,
            definition,
            class,
            kind_type,
            tag,
            attribute,
            property,
            variable,
            operator,
            punctuation,
            regexp,
            escape,
            meta,
            heading,
            link,
        ] = self.0;
        match style {
            SyntaxStyle::Keyword => keyword,
            SyntaxStyle::Comment => comment,
            SyntaxStyle::String => string,
            SyntaxStyle::Number => number,
            SyntaxStyle::Constant => literal,
            SyntaxStyle::Function => function,
            SyntaxStyle::Definition => definition,
            SyntaxStyle::Class => class,
            SyntaxStyle::Type => kind_type,
            SyntaxStyle::Tag => tag,
            SyntaxStyle::Attribute => attribute,
            SyntaxStyle::Property => property,
            SyntaxStyle::Variable => variable,
            SyntaxStyle::Operator => operator,
            SyntaxStyle::Punctuation => punctuation,
            SyntaxStyle::Regexp => regexp,
            SyntaxStyle::Escape => escape,
            SyntaxStyle::Meta => meta,
            SyntaxStyle::Heading => heading,
            SyntaxStyle::Link => link,
        }
    }

    /// Convert only when drawing; the catalogue owns every colour value.
    #[must_use]
    pub fn colour(self, style: SyntaxStyle) -> Hsla {
        gpui::rgb(self.rgb(style)).into()
    }
}

impl SyntaxPalette {
    /// Resolve the chosen syntax brand with only the app light/dark axis.
    #[must_use]
    pub fn colours(self, scheme: Scheme) -> SyntaxColors {
        use palettes::{
            CATPPUCCIN_LATTE, CATPPUCCIN_MOCHA, DRACULA, GITHUB_DARK, GITHUB_LIGHT, GRUVBOX_DARK,
            GRUVBOX_LIGHT, NORD_DARK, NORD_LIGHT, ONE_DARK, ONE_LIGHT, SOLARIZED_DARK,
            SOLARIZED_LIGHT, TOKYO_DAY, TOKYO_NIGHT,
        };
        match (self, scheme) {
            (Self::Github, Scheme::Light) => GITHUB_LIGHT,
            (Self::Github, Scheme::Dark) => GITHUB_DARK,
            (Self::Catppuccin, Scheme::Light) => CATPPUCCIN_LATTE.expand(),
            (Self::Catppuccin, Scheme::Dark) => CATPPUCCIN_MOCHA.expand(),
            (Self::Dracula, Scheme::Light | Scheme::Dark) => DRACULA.expand(),
            (Self::TokyoNight, Scheme::Light) => TOKYO_DAY.expand(),
            (Self::TokyoNight, Scheme::Dark) => TOKYO_NIGHT.expand(),
            (Self::One, Scheme::Light) => ONE_LIGHT.expand(),
            (Self::One, Scheme::Dark) => ONE_DARK.expand(),
            (Self::Nord, Scheme::Light) => NORD_LIGHT.expand(),
            (Self::Nord, Scheme::Dark) => NORD_DARK.expand(),
            (Self::Gruvbox, Scheme::Light) => GRUVBOX_LIGHT.expand(),
            (Self::Gruvbox, Scheme::Dark) => GRUVBOX_DARK.expand(),
            (Self::Solarized, Scheme::Light) => SOLARIZED_LIGHT.expand(),
            (Self::Solarized, Scheme::Dark) => SOLARIZED_DARK.expand(),
        }
    }
}

#[cfg(test)]
mod tests;
