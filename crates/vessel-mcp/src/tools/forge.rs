//! The pull request tools (protocol 11, ADR 0029): an agent reads any
//! session's pull request, and opens one for a session it may operate.
//!
//! [Decision Log]
//! - Opening never pushes: the node would run the repository's hooks and
//!   the user's credentials for the agent, which `git.push` refuses
//!   agents (ADR 0027). The agent pushes with its own tools first; the
//!   node then opens the pull request only when the remote has the branch
//!   at the folder's commit (`not_pushed` otherwise).
//! - Not offered: merging (a person's decision: it publishes to a shared
//!   branch), the feedback switch (what is sent into a session is a
//!   person's to decide), the node's sign-in and the whole list (a
//!   person's view). The node refuses an agent each of them anyway.

use rmcp::model::{JsonObject, Tool};
use serde_json::{Value, json};
use vessel_proto::forge::{GetPullRequest, OpenPullRequest};

use super::{ToolError, VesselTools, flag, required, session, text, tool};

impl VesselTools {
    pub(super) async fn forge(&self, name: &str, args: &JsonObject) -> Result<Value, ToolError> {
        let node = self.target(args).await?;
        let session = session(args)?;
        match name {
            "get_pull_request" => {
                let request = GetPullRequest {
                    session,
                    refresh: flag(args, "refresh")?,
                };
                Ok(json!(self.on(node, request).await?))
            }
            "open_pull_request" => {
                let request = OpenPullRequest {
                    session,
                    title: parse(required(args, "title")?, "title")?,
                    body: text(args, "body")?
                        .map(|body| parse(body, "body"))
                        .transpose()?,
                    base: text(args, "base")?
                        .map(|base| parse(base, "base"))
                        .transpose()?,
                    draft: flag(args, "draft")?,
                };
                Ok(json!(self.on(node, request).await?))
            }
            _ => Err(ToolError::Arguments("unknown pull request tool".into())),
        }
    }
}

fn parse<T: std::str::FromStr>(text: &str, what: &str) -> Result<T, ToolError>
where
    T::Err: std::fmt::Display,
{
    text.parse()
        .map_err(|error| ToolError::Arguments(format!("`{what}`: {error}")))
}

pub(super) fn tools(node: &Value, session: &Value) -> Vec<Tool> {
    vec![
        tool(
            "get_pull_request",
            "A session's pull request (the branch its folder has checked out): its state, \
             checks, reviews, mergeability and head, or why the node cannot see it. `refresh` \
             asks the forge now (only for a session you may operate; at most once per 10 s).",
            json!({
                "node": node,
                "session": session,
                "refresh": { "type": "boolean" }
            }),
            &["session"],
        ),
        tool(
            "open_pull_request",
            "Open a pull request of a session's branch (a session you may operate). Push the \
             branch with your own tools first: the node never pushes for an agent, and answers \
             `not_pushed` when the remote does not have the branch at the folder's commit. An \
             open one of that head is answered `exists`. Merging is a person's.",
            json!({
                "node": node,
                "session": session,
                "title": { "type": "string", "description": "One line." },
                "body": { "type": "string" },
                "base": {
                    "type": "string",
                    "description": "The branch to merge into (default: the project's default \
                                    branch)."
                },
                "draft": { "type": "boolean" }
            }),
            &["session", "title"],
        ),
    ]
}
