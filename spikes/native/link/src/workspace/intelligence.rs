//! Existing read-only node LSP calls with finite deadlines and request
//! correlation. Repository execution/trust remains the node's policy.

#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

use vessel_app_view::workspace::file_ticket::FileTicket;
use vessel_app_view::workspace::intelligence::{IntelligenceAction, IntelligenceFailure};
#[cfg(not(target_arch = "wasm32"))]
use vessel_client::{NodeClient, Unary};
use vessel_proto::{
    lsp,
    workspace::{RelPath, WorkspaceRoot},
};

#[cfg(not(target_arch = "wasm32"))]
use super::calls::call;

/// A cold language server can initialize without blocking the link task.
#[cfg(not(target_arch = "wasm32"))]
const QUERY_DEADLINE: Duration = Duration::from_secs(30);

/// A file-owned query, using only the node's existing LSP request types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntelligenceAsk {
    /// Request identity allocated before submission.
    pub ticket: FileTicket,
    /// Root used by native file ownership and navigation.
    pub root: WorkspaceRoot,
    /// File used by native ownership, independent of the selected tab.
    pub path: RelPath,
    /// Node LSP root/language selection; never a client-local filesystem read.
    pub at: lsp::AtPosition,
    /// The requested closed operation.
    pub action: IntelligenceAction,
}

/// The exact typed answer; no JSON value crosses this boundary.
#[derive(Debug)]
pub enum IntelligenceResult {
    /// Symbol documentation, marked plain text or untrusted Markdown.
    Hover(Result<lsp::Hover, IntelligenceFailure>),
    /// Definition locations under the node's authorized workspace.
    Definitions(Result<lsp::Page<lsp::Location>, IntelligenceFailure>),
    /// Saved-file diagnostics with freshness context and a page cursor.
    Diagnostics(Result<lsp::Page<lsp::Diagnostic>, IntelligenceFailure>),
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn run(client: &NodeClient, ask: &IntelligenceAsk) -> IntelligenceResult {
    match ask.action {
        IntelligenceAction::Hover => {
            IntelligenceResult::Hover(query(client, lsp::GetHover(ask.at.clone())).await)
        }
        IntelligenceAction::Definition => {
            IntelligenceResult::Definitions(query(client, lsp::Definition(ask.at.clone())).await)
        }
        IntelligenceAction::Diagnostics => IntelligenceResult::Diagnostics(
            query(
                client,
                lsp::Diagnostics {
                    workspace: ask.at.workspace.clone(),
                    file: Some(ask.at.file.clone()),
                    page: ask.at.page.clone(),
                },
            )
            .await,
        ),
    }
}

pub(super) fn refused(
    action: IntelligenceAction,
    problem: &vessel_app_view::workspace::problem::Problem,
) -> IntelligenceResult {
    let failure = IntelligenceFailure::Call(problem.clone());
    match action {
        IntelligenceAction::Hover => IntelligenceResult::Hover(Err(failure)),
        IntelligenceAction::Definition => IntelligenceResult::Definitions(Err(failure)),
        IntelligenceAction::Diagnostics => IntelligenceResult::Diagnostics(Err(failure)),
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn query<R: Unary>(
    client: &NodeClient,
    request: R,
) -> Result<R::Response, IntelligenceFailure> {
    tokio::time::timeout(QUERY_DEADLINE, call(client, request))
        .await
        .map_err(|_expired| IntelligenceFailure::Deadline)?
        .map_err(IntelligenceFailure::Call)
}
