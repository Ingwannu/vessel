use super::{Continuity, MAX_SYNTAX_BYTES, decorate};
use std::path::Path;
use vessel_app_kit::code_editor::SyntaxKind;
use vessel_app_kit::output::{AnnotatedLines, Lines};

fn output(text: &str) -> AnnotatedLines {
    Lines::text(text).into()
}

#[test]
fn file_syntax_uses_shared_lexer_state_and_preserves_copy() {
    let text = "/* open\nfn hidden() {}\n*/ fn visible() {}";
    let colored = decorate(
        output(text),
        Some(Path::new(r"C:\src\example.rs")),
        Continuity::File,
    );
    assert!(
        colored.syntax[1]
            .iter()
            .all(|(_, kind)| *kind == Some(SyntaxKind::Comment))
    );
    assert!(
        colored.syntax[2]
            .iter()
            .any(|(_, kind)| *kind == Some(SyntaxKind::Keyword))
    );
    assert_eq!(colored.output.lines.copy.as_deref(), Some(text));
    assert_eq!(
        colored.output.lines.lines[1].text.as_ref(),
        "fn hidden() {}"
    );
    let diff = decorate(
        output(text),
        Some(Path::new("example.rs")),
        Continuity::Diff,
    );
    assert!(
        diff.syntax[1]
            .iter()
            .any(|(_, kind)| *kind == Some(SyntaxKind::Keyword))
    );
}

#[test]
fn a_language_hint_change_changes_semantics_without_reading_the_path() {
    let text = "fn entry() {}";
    let rust = decorate(
        output(text),
        Some(Path::new("missing/example.rs")),
        Continuity::File,
    );
    let plain = decorate(
        output(text),
        Some(Path::new("missing/example.unknown")),
        Continuity::File,
    );
    assert_ne!(rust.syntax, plain.syntax);
    assert_eq!(rust.output.lines.copy, plain.output.lines.copy);
    assert!(
        decorate(output(text), None, Continuity::File)
            .syntax
            .is_empty()
    );
}

#[test]
fn syntax_work_budget_never_truncates_the_output_or_copy_payload() {
    let text = format!(
        "{}\nfn later() {{}}",
        "x".repeat(MAX_SYNTAX_BYTES.saturating_add(1))
    );
    let colored = decorate(
        output(&text),
        Some(Path::new("example.rs")),
        Continuity::File,
    );
    assert!(colored.syntax.iter().all(Vec::is_empty));
    assert_eq!(colored.output.lines.lines.len(), 2);
    assert_eq!(colored.output.lines.copy.as_deref(), Some(text.as_str()));
    let version = vessel_app_kit::version_of;
    assert_ne!(
        version(&(text.as_str(), Path::new("a.rs"))),
        version(&(text.as_str(), Path::new("a.py")))
    );
}
