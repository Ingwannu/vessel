//! Discovery shares the session's correlated request loop; no prompt is sent.

use std::fmt;

use serde_json::Value;

#[cfg(test)]
mod tests;

use super::super::commands::{Catalog, SkillsListParams, SkillsListResponse};
use super::super::wire::RpcError;
use super::{Pending, Session};
use crate::output::AgentOutput;

impl Session {
    pub(super) async fn request_catalog(&mut self) -> bool {
        let params = SkillsListParams {
            cwds: vec![self.setup.cwd.display().to_string()],
        };
        self.request("skills/list", &params, Pending::Catalog).await
    }

    pub(super) async fn catalog_answered(&mut self, answer: Result<Value, RpcError>) {
        match answer.map(serde_json::from_value::<SkillsListResponse>) {
            Ok(Ok(response)) => {
                self.catalog = Catalog::reported(response);
                self.output(AgentOutput::Commands(self.catalog.commands()))
                    .await;
            }
            Ok(Err(error)) => {
                let failure = DecodeFailure(error);
                tracing::warn!(%failure, "Codex skills/list response did not read");
            }
            Err(error) => {
                tracing::info!(code = error.code, "Codex skills/list unavailable");
            }
        }
    }
}

/// Serde retains native values in some errors. Keep the cause for local
/// classification, but end its public chain before input can reach diagnostics.
struct DecodeFailure(serde_json::Error);

impl fmt::Display for DecodeFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "catalog decode {:?} at {}:{}",
            self.0.classify(),
            self.0.line(),
            self.0.column()
        )
    }
}

impl fmt::Debug for DecodeFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DecodeFailure")
            .field("category", &self.0.classify())
            .field("line", &self.0.line())
            .field("column", &self.0.column())
            .finish()
    }
}

impl std::error::Error for DecodeFailure {}
