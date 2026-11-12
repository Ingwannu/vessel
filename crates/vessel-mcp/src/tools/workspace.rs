//! The project and worktree tools (protocol 10, ADR 0027): what an agent
//! needs to start a session in a project or a worktree of its own. The
//! node refuses an agent every other workspace call; the agent reads,
//! writes and runs git with its own tools.
//!
//! [Decision Log]
//! - `create_worktree` follows setup to its end by default (a loop of
//!   `worktree.output` long polls, as `wait_turn` is of `session.read`),
//!   so the agent's next step, a session in it, never meets a setup still
//!   running; it returns setup's state and its last lines.
//! - Commands that need a person's approval are not approved here (the
//!   node refuses an agent `project.approve`): the tool says so, and the
//!   agent can ask for the worktree with `setup: "skip"`.

use std::time::Duration;

use rmcp::model::{JsonObject, Tool};
use serde_json::{Value, json};
use tokio::time::Instant;
use vessel_proto::workspace::{
    CreateWorktree, CreateWorktreeOutcome, ListProjects, ListWorktrees, ReadSetupOutput,
    SessionPlace, SetupChoice, SetupState, WorktreeBase, WorktreeBranch,
};
use vessel_proto::{ProjectId, WorktreeId};

use super::{Target, ToolError, VesselTools, after, flag, number, required, text, tool};

/// How long `create_worktree` follows setup when not told.
const SETUP_WAIT_DEFAULT: Duration = Duration::from_secs(1800);
/// The most of setup's output returned.
const SETUP_TAIL: usize = 4 * 1024;

impl VesselTools {
    pub(super) async fn workspace(
        &self,
        name: &str,
        args: &JsonObject,
    ) -> Result<Value, ToolError> {
        let node = self.target(args).await?;
        match name {
            "list_projects" => Ok(json!(self.on(node, ListProjects {}).await?)),
            "list_worktrees" => {
                let project = text(args, "project")?.map(parse_project).transpose()?;
                Ok(json!(self.on(node, ListWorktrees { project }).await?))
            }
            "create_worktree" => self.create_worktree(node, args).await,
            _ => Err(ToolError::Arguments("unknown workspace tool".into())),
        }
    }

    async fn create_worktree(&self, node: Target, args: &JsonObject) -> Result<Value, ToolError> {
        let project = parse_project(required(args, "project")?)?;
        let branch = match text(args, "branch")? {
            Some(name) if flag(args, "existing")? => WorktreeBranch::Existing {
                name: parse(name, "branch")?,
            },
            name => WorktreeBranch::New {
                name: name.map(|name| parse(name, "branch")).transpose()?,
                base: text(args, "base")?
                    .map(|base| parse(base, "base").map(|name| WorktreeBase::Ref { name }))
                    .transpose()?,
            },
        };
        let setup = match text(args, "setup")? {
            None | Some("run") => SetupChoice::Run,
            Some("skip") => SetupChoice::Skip,
            Some(other) => {
                return Err(ToolError::Arguments(format!(
                    "`setup` is `run` or `skip`, not `{other}`"
                )));
            }
        };
        let outcome = self
            .on(
                node,
                CreateWorktree {
                    project,
                    branch,
                    setup,
                },
            )
            .await?;
        let worktree = match outcome {
            CreateWorktreeOutcome::Created { worktree } => worktree,
            CreateWorktreeOutcome::NeedsApproval { commands } => {
                return Ok(json!({
                    "needs_approval": commands,
                    "note": "A person must approve this project's setup commands in Vessel \
                             before they run; or ask again with `setup: \"skip\"`."
                }));
            }
            CreateWorktreeOutcome::Refused { reason } => return Ok(json!({ "refused": reason })),
        };
        if !matches!(worktree.setup, SetupState::Running { .. }) || flag(args, "no_wait")? {
            return Ok(json!({ "worktree": worktree }));
        }
        let wait = number(args, "timeout_secs")?.map_or(SETUP_WAIT_DEFAULT, Duration::from_secs);
        let (setup, output) = self.follow_setup(node, worktree.id, after(wait)).await?;
        Ok(json!({ "worktree": worktree, "setup": setup, "output": output }))
    }

    /// Follow a worktree's setup until it ends or `deadline`; its state then
    /// and the last of its output.
    async fn follow_setup(
        &self,
        node: Target,
        worktree: WorktreeId,
        deadline: Instant,
    ) -> Result<(SetupState, String), ToolError> {
        let mut from = 0;
        let mut output = String::new();
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let wait = u32::try_from(left.as_secs())
                .unwrap_or(super::POLL_SECS)
                .min(super::POLL_SECS);
            let page = self
                .on(
                    node,
                    ReadSetupOutput {
                        worktree,
                        from,
                        wait: (wait > 0).then_some(wait),
                    },
                )
                .await?;
            output.push_str(&String::from_utf8_lossy(page.bytes.bytes()));
            keep_tail(&mut output);
            let ended = !matches!(page.setup, SetupState::Running { .. });
            if ended && page.next == from {
                return Ok((page.setup, output));
            }
            if !ended && left.is_zero() {
                return Ok((page.setup, output));
            }
            let asked_early = page.next == from && !ended;
            from = page.next;
            if asked_early {
                tokio::time::sleep(super::EARLY_PAUSE).await;
            }
        }
    }
}

/// A session's place from `create_session`'s arguments: a worktree, or a
/// project and a folder in it. A `dir` without its `project`, or a
/// `worktree` with either, is refused rather than left out: the session
/// would start somewhere the agent did not ask for.
pub(super) fn place(args: &JsonObject) -> Result<Option<SessionPlace>, ToolError> {
    let (worktree, project, dir) = (
        text(args, "worktree")?,
        text(args, "project")?,
        text(args, "dir")?,
    );
    match (worktree, project) {
        (Some(worktree), None) if dir.is_none() => Ok(Some(SessionPlace::Worktree {
            worktree: parse(worktree, "worktree")?,
        })),
        (Some(_), _) => Err(ToolError::Arguments(
            "give a `worktree`, or a `project` (and a `dir` in it), not both".to_owned(),
        )),
        (None, Some(project)) => Ok(Some(SessionPlace::Project {
            project: parse_project(project)?,
            dir: dir.map(|dir| parse(dir, "dir")).transpose()?,
        })),
        (None, None) if dir.is_some() => Err(ToolError::Arguments(
            "`dir` is a folder of a `project`; name the project too".to_owned(),
        )),
        (None, None) => Ok(None),
    }
}

fn parse_project(text: &str) -> Result<ProjectId, ToolError> {
    parse(text, "project")
}

fn parse<T: std::str::FromStr>(text: &str, key: &str) -> Result<T, ToolError>
where
    T::Err: std::fmt::Display,
{
    text.parse()
        .map_err(|error| ToolError::Arguments(format!("{key}: {error}")))
}

/// Keep the last [`SETUP_TAIL`] bytes, cut at a character.
fn keep_tail(output: &mut String) {
    let Some(cut) = output.len().checked_sub(SETUP_TAIL) else {
        return;
    };
    let at = (cut..output.len())
        .find(|index| output.is_char_boundary(*index))
        .unwrap_or(output.len());
    drop(output.drain(..at));
}

pub(super) fn tools(node: &Value) -> Vec<Tool> {
    let project =
        json!({ "type": "string", "description": "A project's id, from `list_projects`." });
    vec![
        tool(
            "list_projects",
            "The node's projects (folders and repositories it knows), oldest first, and the git \
             it found.",
            json!({ "node": node }),
            &[],
        ),
        tool(
            "list_worktrees",
            "The node's worktrees, or a project's: each a folder and branch of the project's \
             repository, its setup's state, and the live sessions in it.",
            json!({ "node": node, "project": project }),
            &[],
        ),
        tool(
            "create_worktree",
            "Make a worktree of a project (a folder and a branch of its own) and follow its setup \
             to the end; then start a session in it with `create_session`'s `worktree`. You may \
             have at most 8. Setup commands run only when a person approved them.",
            json!({
                "node": node,
                "project": project,
                "branch": {
                    "type": "string",
                    "description": "The new branch's name (default: vessel/<8 hex>), or with \
                                    `existing` a branch to check out."
                },
                "existing": { "type": "boolean", "description": "Check out `branch` as it is." },
                "base": {
                    "type": "string",
                    "description": "Where a new branch starts (default: the project's default \
                                    branch)."
                },
                "setup": { "type": "string", "enum": ["run", "skip"] },
                "no_wait": { "type": "boolean", "description": "Return without following setup." },
                "timeout_secs": { "type": "integer", "minimum": 1, "maximum": 3600 }
            }),
            &["project"],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tail_keeps_the_end_at_a_character() {
        let mut output = "é".repeat(SETUP_TAIL);
        keep_tail(&mut output);
        assert!(output.len() <= SETUP_TAIL);
        assert!(output.chars().all(|c| c == 'é'));
        let mut short = "short".to_owned();
        keep_tail(&mut short);
        assert_eq!(short, "short");
    }

    #[test]
    fn a_place_is_a_worktree_or_a_project() {
        let worktree = WorktreeId::random();
        let args: JsonObject =
            serde_json::from_value(json!({ "worktree": worktree.to_string() })).unwrap();
        assert_eq!(
            place(&args).unwrap(),
            Some(SessionPlace::Worktree { worktree })
        );
        let project = ProjectId::random();
        let args: JsonObject =
            serde_json::from_value(json!({ "project": project.to_string(), "dir": "src" }))
                .unwrap();
        assert!(matches!(
            place(&args).unwrap(),
            Some(SessionPlace::Project { project: found, dir: Some(_) }) if found == project
        ));
        let args: JsonObject = serde_json::from_value(json!({ "dir": "src" })).unwrap();
        assert!(place(&args).is_err(), "a dir without its project");
        let args: JsonObject = serde_json::from_value(
            json!({ "worktree": worktree.to_string(), "project": project.to_string() }),
        )
        .unwrap();
        assert!(place(&args).is_err(), "a worktree and a project");
        assert_eq!(place(&JsonObject::new()).unwrap(), None);
        let args: JsonObject =
            serde_json::from_value(json!({ "project": project.to_string(), "dir": "../x" }))
                .unwrap();
        assert!(place(&args).is_err());
    }
}
