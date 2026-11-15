//! Saved-source language queries; no language server or UI is started here.

use super::file_ticket::FileTicket;
use vessel_proto::lsp::Language;

/// A requested saved-source query, separate from editor text operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntelligenceAction {
    /// Documentation for the symbol at the caret.
    Hover,
    /// Locations defining the symbol at the caret.
    Definition,
    /// Known diagnostics for this saved document.
    Diagnostics,
}

/// A finite query's failure; timeouts are distinct from a node refusal.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IntelligenceFailure {
    /// The connection/remote node refused the query.
    #[error("the node refused the language query")]
    Call(super::problem::Problem),
    /// No reply within the query deadline.
    #[error("the language query timed out")]
    Deadline,
}

/// Correlation survives tab switches and ignores a replaced query's reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingIntelligence {
    /// Exact process-local request identity.
    pub ticket: FileTicket,
    /// The requested operation.
    pub action: IntelligenceAction,
}

/// Parse a filename extension once at the input edge into a server family.
#[must_use]
pub fn language_for(path: &str) -> Option<Language> {
    let (_, extension) = path.rsplit_once('.')?;
    match extension.to_ascii_lowercase().as_str() {
        "rs" => Some(Language::Rust),
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "mts" | "cts" => Some(Language::TypeScript),
        "py" | "pyi" => Some(Language::Python),
        "go" => Some(Language::Go),
        "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" => Some(Language::Cpp),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_extensions_decode_to_existing_node_server_families() {
        assert_eq!(language_for("src/MAIN.RS"), Some(Language::Rust));
        assert_eq!(language_for("src/a.tsx"), Some(Language::TypeScript));
        assert_eq!(language_for("code.pyi"), Some(Language::Python));
        assert_eq!(language_for("a.cpp"), Some(Language::Cpp));
        assert_eq!(language_for("notes.md"), None);
    }
}
