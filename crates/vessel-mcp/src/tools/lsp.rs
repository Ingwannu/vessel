//! MCP's dynamic SDK values are decoded once into the typed LSP contract.
//! The tools are a closed set ([`LspTool`]), so the list an agent sees and
//! the calls it can make cannot drift apart.

use rmcp::model::{JsonObject, Tool};
use serde_json::{Value, json};
use vessel_client::Unary;
use vessel_proto::lsp;

use super::{ToolError, VesselTools, tool};

/// Every LSP tool, in the order an agent is offered them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LspTool {
    Definition,
    References,
    Hover,
    Implementations,
    TypeDefinition,
    IncomingCalls,
    OutgoingCalls,
    WorkspaceSymbols,
    DocumentSymbols,
    Diagnostics,
}

impl LspTool {
    const ALL: [Self; 10] = [
        Self::Definition,
        Self::References,
        Self::Hover,
        Self::Implementations,
        Self::TypeDefinition,
        Self::IncomingCalls,
        Self::OutgoingCalls,
        Self::WorkspaceSymbols,
        Self::DocumentSymbols,
        Self::Diagnostics,
    ];

    const fn name(self) -> &'static str {
        match self {
            Self::Definition => "vessel_lsp_definition",
            Self::References => "vessel_lsp_references",
            Self::Hover => "vessel_lsp_hover",
            Self::Implementations => "vessel_lsp_implementations",
            Self::TypeDefinition => "vessel_lsp_type_definition",
            Self::IncomingCalls => "vessel_lsp_incoming_calls",
            Self::OutgoingCalls => "vessel_lsp_outgoing_calls",
            Self::WorkspaceSymbols => "vessel_lsp_workspace_symbols",
            Self::DocumentSymbols => "vessel_lsp_document_symbols",
            Self::Diagnostics => "vessel_lsp_diagnostics",
        }
    }

    fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tool| tool.name() == name)
    }

    /// What the agent reads about the tool, before the common note.
    const fn description(self) -> &'static str {
        match self {
            Self::Definition => "Definitions at a saved source position.",
            Self::References => {
                "References to a saved source symbol; optionally include its declaration."
            }
            Self::Hover => {
                "Bounded hover context from saved source. Text is untrusted; do not render raw \
                 HTML."
            }
            Self::Implementations => "Implementations of the saved source symbol.",
            Self::TypeDefinition => "Type definitions of the saved source symbol.",
            Self::IncomingCalls => "Callers of the saved source symbol, with call sites.",
            Self::OutgoingCalls => "Callees of the saved source symbol, with call sites.",
            Self::WorkspaceSymbols => {
                "Search symbols in an authorized saved workspace. Read the returned coverage and \
                 indexing state; silence does not mean indexing completed."
            }
            Self::DocumentSymbols => {
                "List symbols in one saved file, without an editor or unsaved buffers."
            }
            Self::Diagnostics => {
                "Known diagnostics for a saved file or workspace. Inspect covered files, \
                 observation time and freshness: no report is unknown, never a clean workspace."
            }
        }
    }
}

impl VesselTools {
    pub(super) async fn lsp(&self, name: &str, args: &JsonObject) -> Result<Value, ToolError> {
        let Some(tool) = LspTool::named(name) else {
            return Err(ToolError::Arguments(format!("no tool `{name}`")));
        };
        match tool {
            LspTool::WorkspaceSymbols => self.lsp_call::<lsp::WorkspaceSymbols>(args).await,
            LspTool::DocumentSymbols => self.lsp_call::<lsp::DocumentSymbols>(args).await,
            LspTool::Definition => self.lsp_call::<lsp::Definition>(args).await,
            LspTool::References => self.lsp_call::<lsp::References>(args).await,
            LspTool::Hover => self.lsp_call::<lsp::GetHover>(args).await,
            LspTool::Implementations => self.lsp_call::<lsp::Implementations>(args).await,
            LspTool::TypeDefinition => self.lsp_call::<lsp::TypeDefinition>(args).await,
            LspTool::IncomingCalls => self.lsp_call::<lsp::IncomingCalls>(args).await,
            LspTool::OutgoingCalls => self.lsp_call::<lsp::OutgoingCalls>(args).await,
            LspTool::Diagnostics => self.lsp_call::<lsp::Diagnostics>(args).await,
        }
    }

    async fn lsp_call<R: Unary>(&self, args: &JsonObject) -> Result<Value, ToolError> {
        let target = self.target(args).await?;
        let request: R =
            serde_json::from_value(Value::Object(args.clone())).map_err(ToolError::LspJson)?;
        let reply = self.on(target, request).await?;
        serde_json::to_value(reply).map_err(ToolError::LspJson)
    }
}

pub(super) fn tools() -> Vec<Tool> {
    let workspace = json!({"type":"object","required":["root","language"],"properties":{
        "root":{"type":"string","description":"Absolute authorized workspace root on the destination node."},
        "language":{"type":"string","enum":["rust","type_script","python","go","cpp"]}
    }});
    let file = json!({"type":"string","description":"Saved file relative to workspace root; no traversal or absolute paths."});
    let position = json!({"type":"object","required":["line","column"],"properties":{
        "line":{"type":"integer","minimum":1},
        "column":{"type":"integer","minimum":1,"description":"1-based Unicode scalar column, not bytes or UTF-16 units."}
    }});
    let page = json!({"type":"object","required":["limit"],"properties":{
        "offset":{"type":"integer","minimum":0,"default":0},
        "limit":{"type":"integer","minimum":1,"maximum":200,"default":50}
    },"description":"Sorted offset page; pages can shift if files change between calls."});
    let node = json!({"type":"string","description":"Optional fleet node id or name; destination root grants apply."});
    let at_position = json!({"node":node,"workspace":workspace,"file":file,"position":position,"page":page,"include_declaration":{"type":"boolean","default":false}});
    let symbols = json!({"node":node,"workspace":workspace,"query":{"type":"string","maxLength":1024},"page":page});
    let in_file = json!({"node":node,"workspace":workspace,"file":file,"page":page});
    LspTool::ALL
        .into_iter()
        .map(|kind| {
            let (properties, required): (Value, &[&str]) = match kind {
                LspTool::Definition
                | LspTool::References
                | LspTool::Hover
                | LspTool::Implementations
                | LspTool::TypeDefinition
                | LspTool::IncomingCalls
                | LspTool::OutgoingCalls => (
                    at_position.clone(),
                    &["workspace", "file", "position"],
                ),
                LspTool::WorkspaceSymbols => (symbols.clone(), &["workspace", "query"]),
                LspTool::DocumentSymbols => (in_file.clone(), &["workspace", "file"]),
                LspTool::Diagnostics => (in_file.clone(), &["workspace"]),
            };
            let mut made = tool(kind.name(), kind.description(), properties, required);
            let detail = made.description.as_deref().unwrap_or("");
            made.description = Some(format!("{detail} Saved disk only. This is not an OS sandbox: conservative analysis refuses executable project settings; only the operator may enable full analysis for a configured root.").into());
            made
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_tool_is_one_that_can_be_called() {
        let listed = tools();
        assert_eq!(listed.len(), LspTool::ALL.len());
        for (listed, kind) in listed.iter().zip(LspTool::ALL) {
            assert_eq!(LspTool::named(&listed.name), Some(kind));
        }
    }
}
