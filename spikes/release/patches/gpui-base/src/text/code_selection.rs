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

pub(super) fn selected(document: &ParsedDocument, all: bool) -> Option<String> {
    let mut selected = CodeSelection::Unselected;
    for block in document.blocks.iter() {
        selected.merge(block.code_selection(all));
    }
    let CodeSelection::Code(text) = selected else {
        return None;
    };
    let text = text.trim_end_matches(['\r', '\n']);
    (!text.is_empty()).then(|| text.to_owned())
}

impl BlockNode {
    fn code_selection(&self, all: bool) -> CodeSelection {
        match self {
            Self::Root { children, .. } => children_code(children, all),
            Self::Blockquote { children, .. }
            | Self::List { children, .. }
            | Self::ListItem { children, .. } => {
                if all {
                    CodeSelection::Mixed
                } else {
                    children_code(children, false)
                }
            }
            Self::Paragraph(paragraph)
            | Self::Heading {
                children: paragraph,
                ..
            } => paragraph.code_selection(all),
            Self::CodeBlock(code) => {
                let text = if all {
                    code.code().to_string()
                } else {
                    self.selected_text(SelectionFormat::Plain)
                };
                if text.is_empty() {
                    CodeSelection::Unselected
                } else {
                    CodeSelection::Code(text)
                }
            }
            Self::Table(table) => {
                if all {
                    return CodeSelection::Mixed;
                }
                let mut selected = CodeSelection::Unselected;
                for row in &table.children {
                    for cell in &row.children {
                        selected.merge(cell.children.code_selection(false));
                    }
                }
                selected
            }
            Self::Custom { .. } => {
                if all || self.has_selection() {
                    CodeSelection::Mixed
                } else {
                    CodeSelection::Unselected
                }
            }
            Self::HorizontalRule { .. } | Self::Unknown => {
                if all {
                    CodeSelection::Mixed
                } else {
                    CodeSelection::Unselected
                }
            }
            Self::Definition { .. } | Self::Break { .. } => CodeSelection::Unselected,
        }
    }
}

fn children_code(children: &[BlockNode], all: bool) -> CodeSelection {
    let mut selected = CodeSelection::Unselected;
    for child in children {
        selected.merge(child.code_selection(all));
    }
    selected
}
