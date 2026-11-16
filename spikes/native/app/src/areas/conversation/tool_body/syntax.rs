//! File syntax decorates existing output lines without file I/O or another lexer.
use std::path::Path;
use vessel_app_kit::code_editor::SyntaxKind;
use vessel_app_kit::output::{AnnotatedLines, SyntaxLines};
use vessel_app_view::workspace::syntax::{LineState, language_of, tokenize};

/// Syntax decoration alone is bounded; displayed text and copy remain complete.
const MAX_SYNTAX_BYTES: usize = 65_536;

#[derive(Clone, Copy)]
pub(super) enum Continuity {
    File,
    Diff,
}

pub(super) fn decorate(
    output: AnnotatedLines,
    path: Option<&Path>,
    continuity: Continuity,
) -> SyntaxLines {
    let Some(path) = path.and_then(Path::to_str) else {
        return output.into();
    };
    // A provider path is only a language hint. Neither normalization nor an open
    // request is performed, including for paths with another platform's slashes.
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let language = language_of(name);
    let mut state = LineState::Code;
    let mut remaining = MAX_SYNTAX_BYTES;
    let mut exhausted = false;
    let syntax = output
        .lines
        .lines
        .iter()
        .map(|line| {
            if exhausted || line.text.len() > remaining {
                exhausted = true;
                return Vec::new();
            }
            remaining = remaining.saturating_sub(line.text.len());
            let initial = match continuity {
                Continuity::File => state,
                Continuity::Diff => LineState::Code,
            };
            let (tokens, next) = tokenize(&line.text, language, initial);
            state = next;
            tokens
                .into_iter()
                .map(|token| {
                    (
                        token.range,
                        Some(
                            token
                                .kind
                                .map_or(SyntaxKind::Variable, crate::syntax_kind::of),
                        ),
                    )
                })
                .collect()
        })
        .collect();
    SyntaxLines { output, syntax }
}

#[cfg(test)]
mod tests;
