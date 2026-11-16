//! The wrapper routes one existing lexer, never defines a product highlighter.
use super::*;
use crate::code_editor::SyntaxKind;

struct Comments;
impl LineSyntax for Comments {
    type State = bool;
    fn language(&self) -> gpui::SharedString {
        "fixture".into()
    }
    fn line(&self, text: &str, open: bool) -> (Runs, bool) {
        let opening = text == "/*";
        let closing = text == "*/";
        let kind = if open || opening {
            SyntaxKind::Comment
        } else {
            SyntaxKind::Keyword
        };
        (
            vec![(0..text.len(), Some(kind))],
            !closing && (open || opening),
        )
    }
}
fn lexer(side: DiffSide, rows: &[DiffTone]) -> DiffSyntax<Comments> {
    DiffSyntax::new(
        Comments,
        rows.iter()
            .map(|tone| DiffLine {
                number: None,
                tone: *tone,
            })
            .collect::<Vec<_>>()
            .into(),
        side,
    )
}

#[test]
fn a_removed_comment_does_not_poison_added_or_modified_context_syntax() {
    let rows = [DiffTone::Removed, DiffTone::Added, DiffTone::Context];
    for (side, context) in [
        (DiffSide::Original, SyntaxKind::Comment),
        (DiffSide::Modified, SyntaxKind::Keyword),
    ] {
        let lexer = lexer(side, &rows);
        assert_eq!(lexer.language(), gpui::SharedString::from("fixture"));
        let (removed, state) = lexer.line("/*", State::default());
        let (added, state) = lexer.line("new", state);
        let (following, _state) = lexer.line("same", state);
        assert_eq!(removed, vec![(0..2, Some(SyntaxKind::Comment))]);
        assert_eq!(added, vec![(0..3, Some(SyntaxKind::Keyword))]);
        assert_eq!(following, vec![(0..4, Some(context))]);
    }
}

#[test]
fn headers_reset_unknown_context_while_empty_and_notice_rows_do_not_lex() {
    let lexer = lexer(
        DiffSide::Modified,
        &[
            DiffTone::Added,
            DiffTone::Empty,
            DiffTone::Notice,
            DiffTone::Context,
            DiffTone::Header,
            DiffTone::Context,
        ],
    );
    let (_, state) = lexer.line("/*", State::default());
    let (empty, state) = lexer.line("", state);
    let (notice, state) = lexer.line("not code", state);
    let (before, state) = lexer.line("same", state);
    let (header, state) = lexer.line("@@ -20,1 +30,1 @@", state);
    let (after, state) = lexer.line("next", state);
    assert_eq!(
        (empty, notice, header),
        (Vec::new(), Vec::new(), Vec::new())
    );
    assert_eq!(before, vec![(0..4, Some(SyntaxKind::Comment))]);
    assert_eq!(after, vec![(0..4, Some(SyntaxKind::Keyword))]);
    let (beyond, _state) = lexer.line("not a source row", state);
    assert_eq!(beyond, Vec::new());
}
