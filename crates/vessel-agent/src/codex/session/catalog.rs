//! Discovery shares the session's correlated request loop; no prompt is sent.

use serde_json::Value;

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
                tracing::warn!(%error, "Codex skills/list response did not read");
            }
            Err(error) => {
                tracing::info!(code = error.code, "Codex skills/list unavailable");
            }
        }
    }
}
