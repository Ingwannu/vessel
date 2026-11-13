//! Syntax colour for a diff's lines and a file's, by a small lexer:
//! comments, strings, numbers, keywords, constants and called names,
//! coloured with the design system's syntax tokens (DESIGN.md 4.4). The
//! page's `data/workspace/syntax.ts`, ported with its cases, so a file
//! reads in the same colours on every surface.
//!
//! [Decision Log]
//! - Purpose: a diff or a code viewer needs syntax colour.
//! - Constraint: one experience everywhere (ADR 0036): the page colours
//!   with this lexer, so a different highlighter here would colour the
//!   same file differently. Lines are drawn a window at a time, so
//!   colouring must be cheap per line.
//! - Alternatives: tree-sitter (MIT; exact grammars, what Zed and
//!   `gpui-component`'s highlighter use: a C grammar per language compiled
//!   into the app, megabytes, and colours that differ from the page's
//!   until the page uses it too); `syntect` (MIT; `TextMate` grammars, the
//!   same difference and a regex engine at start-up).
//! - Choice: the page's lexer, rule for rule: a line is read from the state
//!   the line before left (inside a block comment or not), so a file is
//!   read once in order; a diff's lines are read each on its own. A token
//!   is a byte range of the line, so colouring allocates one small list.
//! - Cost: no grammar's precision (a regular expression literal, a
//!   template's nested expression, a heredoc are not told apart), and a
//!   block comment opened outside a diff's hunk is not coloured in it. If
//!   the page moves to grammars, both move together.

mod languages;

use std::ops::Range;

pub use languages::{Language, language_of};

/// What a run of text is, for its colour (`None` on a token: plain).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    /// A keyword.
    Keyword,
    /// A string.
    String,
    /// A comment.
    Comment,
    /// A number.
    Number,
    /// A constant (`true`, `None`).
    Constant,
    /// A called name, or markup's attribute.
    Function,
    /// Markup's tag.
    Tag,
}

/// A run of a line's text: its bytes in the line, and what it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// Where it is in the line.
    pub range: Range<usize>,
    /// What it is (`None`: plain).
    pub kind: Option<TokenKind>,
}

/// What a line leaves open for the next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineState {
    /// Code.
    #[default]
    Code,
    /// Inside a block comment.
    Comment,
}

/// Markup's comment marks.
const MARKUP_COMMENT: (&str, &str) = ("<!--", "-->");

/// Tokens being gathered, a run joined to the one before when of one kind.
struct Runs(Vec<Token>);

impl Runs {
    fn push(&mut self, range: Range<usize>, kind: Option<TokenKind>) {
        if range.is_empty() {
            return;
        }
        if let Some(last) = self.0.last_mut()
            && last.kind == kind
            && last.range.end == range.start
        {
            last.range.end = range.end;
            return;
        }
        self.0.push(Token { range, kind });
    }
}

/// The tokens of one line of `language`, read from `state`, and the state
/// it leaves.
#[must_use]
pub fn tokenize(text: &str, language: &Language, state: LineState) -> (Vec<Token>, LineState) {
    let mut runs = Runs(Vec::new());
    let end = text.len();
    let block = if language.markup {
        Some(MARKUP_COMMENT)
    } else {
        language.block
    };
    let mut index = 0_usize;
    if state == LineState::Comment
        && let Some((_, close)) = block
    {
        let Some(found) = text.find(close) else {
            runs.push(0..end, Some(TokenKind::Comment));
            return (runs.0, LineState::Comment);
        };
        index = found.saturating_add(close.len());
        runs.push(0..index, Some(TokenKind::Comment));
    }
    while let Some(rest) = text.get(index..).filter(|rest| !rest.is_empty()) {
        let Some(character) = rest.chars().next() else {
            break;
        };
        if language.line.iter().any(|mark| rest.starts_with(mark)) {
            runs.push(index..end, Some(TokenKind::Comment));
            break;
        }
        if let Some((open, close)) = block
            && rest.starts_with(open)
        {
            let from = index.saturating_add(open.len());
            let Some(found) = text.get(from..).and_then(|after| after.find(close)) else {
                runs.push(index..end, Some(TokenKind::Comment));
                return (runs.0, LineState::Comment);
            };
            let after = from.saturating_add(found).saturating_add(close.len());
            runs.push(index..after, Some(TokenKind::Comment));
            index = after;
            continue;
        }
        if language.quotes.contains(character) && !(language.markup && character == '\'') {
            let after = closing(text, index, character);
            runs.push(index..after, Some(TokenKind::String));
            index = after;
            continue;
        }
        if language.markup
            && character == '<'
            && let Some(length) = tag_length(rest)
        {
            let after = index.saturating_add(length);
            runs.push(index..after, Some(TokenKind::Tag));
            index = after;
            continue;
        }
        if character.is_ascii_digit() {
            let after = index.saturating_add(number_length(rest));
            runs.push(index..after, Some(TokenKind::Number));
            index = after;
            continue;
        }
        if let Some(length) = word_length(rest) {
            let after = index.saturating_add(length);
            let word = text.get(index..after).unwrap_or_default();
            let next = text.get(after..).and_then(|rest| rest.chars().next());
            runs.push(index..after, word_kind(word, next, language));
            index = after;
            continue;
        }
        let after = index.saturating_add(character.len_utf8());
        runs.push(index..after, None);
        index = after;
    }
    (runs.0, LineState::Code)
}

/// What a word is: a keyword, a constant, a called name, or plain.
fn word_kind(word: &str, next: Option<char>, language: &Language) -> Option<TokenKind> {
    if language.markup {
        return (next == Some('=')).then_some(TokenKind::Function);
    }
    let lowered;
    let key = if language.any_case {
        lowered = word.to_lowercase();
        lowered.as_str()
    } else {
        word
    };
    if language.keywords.contains(key) {
        Some(TokenKind::Keyword)
    } else if language.constants.contains(key) {
        Some(TokenKind::Constant)
    } else {
        (next == Some('(')).then_some(TokenKind::Function)
    }
}

/// Where the string opened by `quote` at `start` ends (after its closing
/// quote, or the line's end): a backslash escapes the next character.
fn closing(text: &str, start: usize, quote: char) -> usize {
    let from = start.saturating_add(quote.len_utf8());
    let mut characters = text.get(from..).unwrap_or_default().char_indices();
    while let Some((at, character)) = characters.next() {
        if character == '\\' {
            let _escaped = characters.next();
            continue;
        }
        if character == quote {
            return from.saturating_add(at).saturating_add(character.len_utf8());
        }
    }
    text.len()
}

/// How many leading bytes of `rest` are taken by `predicate`.
fn run_of(rest: &[u8], predicate: impl Fn(u8) -> bool) -> usize {
    rest.iter().take_while(|&&byte| predicate(byte)).count()
}

/// `[\w$-]`, the word's later characters.
const fn word_tail(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$' || byte == b'-'
}

/// A word: `[A-Za-z_$@][\w$-]*`.
fn word_length(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    let first = *bytes.first()?;
    if !(first.is_ascii_alphabetic() || first == b'_' || first == b'$' || first == b'@') {
        return None;
    }
    let tail = bytes.get(1..).unwrap_or_default();
    Some(run_of(tail, word_tail).saturating_add(1))
}

/// A tag: `</?[A-Za-z][\w:.-]*`.
fn tag_length(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    let mut at = 1_usize;
    if bytes.get(at) == Some(&b'/') {
        at = at.saturating_add(1);
    }
    if !bytes.get(at)?.is_ascii_alphabetic() {
        return None;
    }
    at = at.saturating_add(1);
    let tail = bytes.get(at..).unwrap_or_default();
    let length = run_of(tail, |byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':' | b'.' | b'-')
    });
    Some(at.saturating_add(length))
}

/// A number from a digit: `0x` hex, `0b` binary, or a decimal with a
/// fraction and an exponent, then any letters or `%` (a suffix, a unit).
fn number_length(rest: &str) -> usize {
    let bytes = rest.as_bytes();
    let tail = |from: usize| bytes.get(from..).unwrap_or_default();
    let prefixed = |marks: [u8; 2], digit: fn(u8) -> bool| -> Option<usize> {
        if bytes.first() != Some(&b'0') || !bytes.get(1).is_some_and(|byte| marks.contains(byte)) {
            return None;
        }
        let length = run_of(tail(2), digit);
        (length > 0).then(|| length.saturating_add(2))
    };
    let body = prefixed(*b"xX", |byte| byte.is_ascii_hexdigit() || byte == b'_')
        .or_else(|| prefixed(*b"bB", |byte| matches!(byte, b'0' | b'1' | b'_')))
        .unwrap_or_else(|| {
            let decimal = |byte: u8| byte.is_ascii_digit() || byte == b'_';
            let mut at = run_of(bytes, decimal);
            if tail(at).first() == Some(&b'.')
                && tail(at.saturating_add(1))
                    .first()
                    .is_some_and(u8::is_ascii_digit)
            {
                at = at.saturating_add(1);
                at = at.saturating_add(run_of(tail(at), decimal));
            }
            if tail(at)
                .first()
                .is_some_and(|byte| matches!(byte, b'e' | b'E'))
            {
                let sign = usize::from(
                    tail(at.saturating_add(1))
                        .first()
                        .is_some_and(|byte| matches!(byte, b'+' | b'-')),
                );
                let from = at.saturating_add(1).saturating_add(sign);
                let digits = run_of(tail(from), |byte| byte.is_ascii_digit());
                if digits > 0 {
                    at = from.saturating_add(digits);
                }
            }
            at
        });
    let suffix = run_of(tail(body), |byte| {
        byte.is_ascii_alphabetic() || byte == b'%'
    });
    body.saturating_add(suffix)
}

/// The state each line of `lines` starts in, read in order: what a windowed
/// viewer needs to colour any line without the ones before it. One pass.
#[must_use]
pub fn line_states<'a>(
    lines: impl IntoIterator<Item = &'a str>,
    language: &Language,
) -> Vec<LineState> {
    let mut states = Vec::new();
    let mut state = LineState::Code;
    let carries = language.block.is_some() || language.markup;
    for line in lines {
        states.push(state);
        state = if carries {
            tokenize(line, language, state).1
        } else {
            LineState::Code
        };
    }
    states
}

#[cfg(test)]
mod tests {
    //! The page's `syntax.test.ts`, case for case.

    use super::*;

    /// The coloured runs of `line`, as `kind:text`.
    fn runs(line: &str, path: &str) -> Vec<String> {
        tokenize(line, language_of(path), LineState::Code)
            .0
            .into_iter()
            .filter_map(|token| {
                let kind = match token.kind? {
                    TokenKind::Keyword => "keyword",
                    TokenKind::String => "string",
                    TokenKind::Comment => "comment",
                    TokenKind::Number => "number",
                    TokenKind::Constant => "constant",
                    TokenKind::Function => "function",
                    TokenKind::Tag => "tag",
                };
                Some(format!("{kind}:{}", line.get(token.range)?))
            })
            .collect()
    }

    #[test]
    fn chooses_the_language_by_the_file_s_name() {
        assert_eq!(language_of("src/app.tsx").name, "typescript");
        assert_eq!(language_of("crates/x/src/lib.rs").name, "rust");
        assert_eq!(language_of("Dockerfile").name, "shell");
        assert_eq!(language_of("notes.unknownext").name, "text");
    }

    #[test]
    fn colours_keywords_strings_numbers_calls_and_comments() {
        assert_eq!(
            runs(r#"const x = load("a.json", 42); // why"#, "a.ts"),
            [
                "keyword:const",
                "function:load",
                r#"string:"a.json""#,
                "number:42",
                "comment:// why"
            ]
        );
    }

    #[test]
    fn ends_a_string_at_its_quote_not_at_an_escaped_one() {
        assert_eq!(
            runs(r#"let s = "a \" b"; true"#, "a.rs"),
            ["keyword:let", r#"string:"a \" b""#, "constant:true"]
        );
    }

    #[test]
    fn carries_a_block_comment_to_the_next_line() {
        let language = language_of("a.go");
        let (_, first) = tokenize("x := 1 /* starts", language, LineState::Code);
        assert_eq!(first, LineState::Comment);
        let line = "still */ y";
        let (tokens, _) = tokenize(line, language, first);
        assert_eq!(line.get(tokens[0].range.clone()), Some("still */"));
        assert_eq!(tokens[0].kind, Some(TokenKind::Comment));
        assert_eq!(
            line_states(["/* a", "b */", "c"], language),
            [LineState::Code, LineState::Comment, LineState::Code]
        );
    }

    #[test]
    fn reads_sql_keywords_in_any_case_and_markup_s_tags() {
        assert_eq!(
            runs("SELECT id FROM t", "q.sql"),
            ["keyword:SELECT", "keyword:FROM"]
        );
        assert_eq!(
            runs(r#"<a href="x">"#, "p.html"),
            ["tag:<a", "function:href", r#"string:"x""#]
        );
    }

    #[test]
    fn keeps_a_line_s_text_whole() {
        let line = r#"  fn main() { println!("hi"); } // done"#;
        let (tokens, _) = tokenize(line, language_of("main.rs"), LineState::Code);
        let joined: String = tokens
            .iter()
            .filter_map(|token| line.get(token.range.clone()))
            .collect();
        assert_eq!(joined, line);
    }

    #[test]
    fn reads_numbers_as_the_page_s_pattern_does() {
        assert_eq!(
            runs("0xff_u8 0b10 1.5e-3 12px 3.x", "a.rs"),
            [
                "number:0xff_u8",
                "number:0b10",
                "number:1.5e-3",
                "number:12px",
                "number:3",
            ]
        );
    }

    #[test]
    fn steps_over_a_character_outside_ascii() {
        let line = "let é = \"ü\\é\";";
        let (tokens, _) = tokenize(line, language_of("a.rs"), LineState::Code);
        let joined: String = tokens
            .iter()
            .filter_map(|token| line.get(token.range.clone()))
            .collect();
        assert_eq!(joined, line);
    }
}
