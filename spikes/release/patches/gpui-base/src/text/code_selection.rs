// MODIFIED by Vessel: code-only copy queries the existing parsed selection.
//! No source scan: inline marks and code blocks already know the rendered code.

use super::{SelectionFormat, document::ParsedDocument, node::BlockNode};

#[derive(Default)]
pub(super) enum CodeSelection {
    #[default]
    Unselected,
    Code(String),
    Mixed,
}
impl CodeSelection {
    fn merge(&mut self, other: Self) {
        *self = match (std::mem::take(self), other) {
            (Self::Unselected, value) | (value, Self::Unselected) => value,
            _ => Self::Mixed,
        };
    }
}

pub(super) fn selected(document: &ParsedDocument) -> Option<String> {
    let mut selected = CodeSelection::Unselected;
    for block in document.blocks.iter() {
        selected.merge(block.code_selection());
    }
    let CodeSelection::Code(text) = selected else {
        return None;
    };
    let text = trim_code_line_breaks(&text);
    (!text.is_empty()).then(|| text.to_owned())
}

/// Remove trailing LF/CRLF lines and their following ASCII indentation.
///
/// Retain spaces on the final nonempty line and every leading code byte. Both
/// the selection query and host code-copy buttons use this operation, so pasting
/// code cannot unexpectedly execute a final line. A lone CR remains literal.
#[must_use]
pub fn trim_code_line_breaks(mut text: &str) -> &str {
    loop {
        let candidate = text.trim_end_matches([' ', '\t']);
        let Some(without_lf) = candidate.strip_suffix('\n') else {
            return text;
        };
        text = without_lf.strip_suffix('\r').unwrap_or(without_lf);
    }
}

impl BlockNode {
    fn code_selection(&self) -> CodeSelection {
        match self {
            Self::Root { children, .. }
            | Self::Blockquote { children, .. }
            | Self::List { children, .. }
            | Self::ListItem { children, .. } => {
                let mut selected = CodeSelection::Unselected;
                for child in children {
                    selected.merge(child.code_selection());
                }
                selected
            }
            Self::Paragraph(paragraph)
            | Self::Heading {
                children: paragraph,
                ..
            } => paragraph.code_selection(),
            Self::CodeBlock(_) => {
                let text = self.selected_text(SelectionFormat::Plain);
                if text.is_empty() {
                    CodeSelection::Unselected
                } else {
                    CodeSelection::Code(text)
                }
            }
            Self::Table(table) => {
                let mut selected = CodeSelection::Unselected;
                for row in &table.children {
                    for cell in &row.children {
                        selected.merge(cell.children.code_selection());
                    }
                }
                selected
            }
            Self::Custom { .. } => {
                if self.has_selection() {
                    CodeSelection::Mixed
                } else {
                    CodeSelection::Unselected
                }
            }
            Self::HorizontalRule { .. }
            | Self::Unknown
            | Self::Definition { .. }
            | Self::Break { .. } => CodeSelection::Unselected,
        }
    }
}
