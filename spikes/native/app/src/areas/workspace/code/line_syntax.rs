//! The view's lexer as the code editor's line syntax, so a file edited is
//! coloured exactly as the same file viewed (`viewer.rs` draws its lines
//! with the same lexer and the same code colours).

use vessel_app_kit::code_editor::{LineSyntax, Runs};
use vessel_app_kit::ui::SharedString;
use vessel_app_view::workspace::syntax::{Language, LineState, tokenize};

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
            .map(|token| (token.range, token.kind.map(crate::syntax_kind::of)))
            .collect();
        (runs, end)
    }
}
