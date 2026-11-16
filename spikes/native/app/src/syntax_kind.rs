//! The sole semantic lexer-to-kit bridge, shared by editor and readonly preview.
use vessel_app_kit::code_editor::SyntaxKind;
use vessel_app_view::workspace::syntax::TokenKind;

/// The kit's kind for the lexer's.
pub(crate) const fn of(kind: TokenKind) -> SyntaxKind {
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
