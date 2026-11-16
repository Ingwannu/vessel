//! The view's lexer as the code editor's line syntax, so a file edited is
//! coloured exactly as the same file viewed (`viewer.rs` draws its lines
//! with the same lexer and the same code colours).

use vessel_app_kit::code_editor::{LineSyntax, Runs, SyntaxKind};
use vessel_app_kit::ui::SharedString;
use vessel_app_view::workspace::syntax::{Language, LineState, TokenKind, tokenize};

/// The diff uses the exact source-editor lexer and seven-kind mapping.
pub(crate) fn syntax_for(path: &vessel_proto::workspace::RelPath) -> impl LineSyntax {
    ViewSyntax(vessel_app_view::workspace::syntax::language_of(
        path.as_str(),
    ))
}

/// A file's language, for the editor.
pub(super) struct ViewSyntax(pub(super) &'static Language);

impl LineSyntax for ViewSyntax {
    type State = LineState;

    fn language(&self) -> SharedString {
        SharedString::new_static(self.0.name)
    }

    fn line(&self, text: &str, state: LineState) -> (Runs, LineState) {
        let (tokens, end) = tokenize(text, self.0, state);
        let runs = tokens
            .into_iter()
            .map(|token| (token.range, token.kind.map(kind_of)))
            .collect();
        (runs, end)
    }
}

/// The kit's kind for the lexer's.
const fn kind_of(kind: TokenKind) -> SyntaxKind {
    match kind {
        TokenKind::Keyword => SyntaxKind::Keyword,
        TokenKind::String => SyntaxKind::String,
        TokenKind::Comment => SyntaxKind::Comment,
        TokenKind::Number => SyntaxKind::Number,
        TokenKind::Constant => SyntaxKind::Constant,
        TokenKind::Function => SyntaxKind::Function,
        TokenKind::Tag => SyntaxKind::Tag,
    }
}
