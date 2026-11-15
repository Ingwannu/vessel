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
    let text = text.trim_end_matches(['\r', '\n']);
    (!text.is_empty()).then(|| text.to_owned())
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
