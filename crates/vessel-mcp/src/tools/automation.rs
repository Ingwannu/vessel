//! The schedule and saved-prompt tools (protocol 11, ADR 0033), on the
//! agent's own node only: a schedule acts as the agent who made it, and
//! another node cannot rebuild that agent when the run comes (the node
//! refuses a delegated caller's create).
//!
//! What an agent may do is the node's rule: eight schedules per session,
//! a new session's target or one it owns or its own session (a
//! heartbeat), deleted with its session; never run one now; saved prompts
//! read, not written. No `run_schedule` and no hook tool, for those
//! reasons.

use rmcp::model::{JsonObject, Tool};
use serde_json::{Value, json};
use vessel_proto::automation::{
    AfterRun, Cadence, CatchUp, EverySeconds, ListPrompts, MaxRuns, ScheduleDefinition,
    ScheduleTarget,
};
use vessel_proto::{ProjectId, ScheduleId};

use super::{ToolError, VesselTools, flag, number, required, text, tool};

impl VesselTools {
    pub(super) async fn automation(
        &self,
        name: &str,
        args: &JsonObject,
    ) -> Result<Value, ToolError> {
        match name {
            "create_schedule" => {
                let target = match text(args, "session")? {
                    Some(session) => ScheduleTarget::Session {
                        session: parse(session, "session")?,
                    },
                    None => ScheduleTarget::NewSession {
                        spec: Box::new(Self::spec(args)?),
                        after: if flag(args, "keep")? {
                            AfterRun::Keep
                        } else {
                            AfterRun::Archive
                        },
                    },
                };
                let definition = definition(args, target)?;
                Ok(json!(self.client.create_schedule(definition, false).await?))
            }
            "create_heartbeat" => {
                let session = self.own_session().ok_or_else(|| {
                    ToolError::Arguments("this tool server does not know its session".to_owned())
                })?;
                let definition = definition(args, ScheduleTarget::Session { session })?;
                Ok(json!(self.client.create_schedule(definition, false).await?))
            }
            "list_schedules" => Ok(json!({ "schedules": self.client.schedules().await? })),
            "schedule_runs" => Ok(json!({
                "runs": self.client.schedule_runs(schedule(args)?).await?
            })),
            "pause_schedule" => Ok(json!(self.client.pause_schedule(schedule(args)?).await?)),
            "resume_schedule" => Ok(json!(self.client.resume_schedule(schedule(args)?).await?)),
            "delete_schedule" => {
                let schedule = schedule(args)?;
                self.client.delete_schedule(schedule).await?;
                Ok(json!({ "deleted": schedule }))
            }
            "list_saved_prompts" => {
                let project = text(args, "project")?
                    .map(|project| parse::<ProjectId>(project, "project"))
                    .transpose()?;
                Ok(json!(self.client.call(ListPrompts { project }).await?))
            }
            _ => Err(ToolError::Arguments("unknown schedule tool".into())),
        }
    }
}

/// A schedule's definition from a tool's arguments: `every` (seconds) or
/// `cron` with `tz`, `prompt`, and the optional rest.
fn definition(args: &JsonObject, target: ScheduleTarget) -> Result<ScheduleDefinition, ToolError> {
    let cadence = match (number(args, "every_secs")?, text(args, "cron")?) {
        (Some(seconds), None) => Cadence::Every {
            seconds: u32::try_from(seconds)
                .ok()
                .and_then(|seconds| EverySeconds::try_from(seconds).ok())
                .ok_or_else(|| {
                    ToolError::Arguments(
                        "`every_secs` is from 60 (a minute) to 31622400 (a year)".to_owned(),
                    )
                })?,
            anchor: None,
        },
        (None, Some(cron)) => Cadence::Cron {
            expr: parse(cron, "cron")?,
            zone: parse(required(args, "tz")?, "tz")?,
        },
        _ => {
            return Err(ToolError::Arguments(
                "give `every_secs` or `cron` (with `tz`), not both".to_owned(),
            ));
        }
    };
    let catch_up = match text(args, "catch_up")? {
        None | Some("once") => CatchUp::Once,
        Some("skip") => CatchUp::Skip,
        Some(other) => {
            return Err(ToolError::Arguments(format!(
                "`catch_up` is `once` or `skip`, not `{other}`"
            )));
        }
    };
    let max_runs = number(args, "max_runs")?
        .map(|runs| {
            u32::try_from(runs)
                .ok()
                .and_then(|runs| MaxRuns::try_from(runs).ok())
                .ok_or_else(|| ToolError::Arguments("`max_runs` is at least 1".to_owned()))
        })
        .transpose()?;
    Ok(ScheduleDefinition {
        name: text(args, "name")?
            .map(|name| parse(name, "name"))
            .transpose()?,
        cadence,
        target,
        prompt: parse(required(args, "prompt")?, "prompt")?,
        catch_up,
        max_runs,
        ends_at: text(args, "ends_at")?
            .map(|at| parse(at, "ends_at"))
            .transpose()?,
    })
}

fn schedule(args: &JsonObject) -> Result<ScheduleId, ToolError> {
    parse(required(args, "schedule")?, "schedule")
}

fn parse<T: std::str::FromStr>(text: &str, what: &str) -> Result<T, ToolError>
where
    T::Err: std::fmt::Display,
{
    text.parse()
        .map_err(|error| ToolError::Arguments(format!("{what}: {error}")))
}

/// The tools, with their schemas.
pub(super) fn tools() -> Vec<Tool> {
    let schedule = json!({ "type": "string", "description": "A schedule id." });
    let when = |extra: Value| {
        let mut properties = json!({
            "name": { "type": "string", "description": "What to call it." },
            "every_secs": {
                "type": "integer", "minimum": 60,
                "description": "Run every so many seconds (or give `cron`)."
            },
            "cron": {
                "type": "string",
                "description": "Five-field cron expression, or `@daily` and its kind."
            },
            "tz": {
                "type": "string",
                "description": "With `cron`: an IANA time zone, such as Europe/London."
            },
            "prompt": { "type": "string", "description": "The prompt each run sends." },
            "max_runs": { "type": "integer", "minimum": 1, "description": "End after this many." },
            "ends_at": { "type": "string", "description": "End at this RFC 3339 time." },
            "catch_up": {
                "type": "string", "enum": ["once", "skip"],
                "description": "Due times missed while the node was stopped: one run for the \
                                latest (if less than a day old), or none."
            }
        });
        if let (Some(all), Value::Object(more)) = (properties.as_object_mut(), extra) {
            all.extend(more);
        }
        properties
    };
    vec![
        tool(
            "create_schedule",
            "Prompt a session on a schedule, on this node: a new session each run (`harness`, \
             with `profile`, `project` or `worktree`, `model`; archived after its first turn \
             unless `keep`), or a session of yours through its queue (`session`). Runs act as \
             you, and your schedules end when your session does.",
            when(json!({
                "session": { "type": "string", "description": "A session of yours to prompt." },
                "harness": { "type": "string", "description": "claude, codex, gemini or opencode." },
                "profile": { "type": "string", "description": "One of the node's profiles." },
                "project": { "type": "string", "description": "A project id (`list_projects`)." },
                "dir": { "type": "string", "description": "With `project`: a folder in it." },
                "worktree": { "type": "string", "description": "A worktree of yours." },
                "model": { "type": "string", "description": "A model the agent offers." },
                "keep": { "type": "boolean", "description": "Keep each run's session." }
            })),
            &["prompt"],
        ),
        tool(
            "create_heartbeat",
            "Prompt your own session on a schedule, through its queue (never into a running \
             turn): a reminder to yourself that ends when your session does.",
            when(json!({})),
            &["prompt"],
        ),
        tool(
            "list_schedules",
            "The node's schedules, newest first.",
            json!({}),
            &[],
        ),
        tool(
            "schedule_runs",
            "A schedule's newest runs (at most 50) and what each came to.",
            json!({ "schedule": schedule }),
            &["schedule"],
        ),
        tool(
            "pause_schedule",
            "Stop one of your schedules until `resume_schedule`.",
            json!({ "schedule": schedule }),
            &["schedule"],
        ),
        tool(
            "resume_schedule",
            "Let one of your paused schedules run again, from now.",
            json!({ "schedule": schedule }),
            &["schedule"],
        ),
        tool(
            "delete_schedule",
            "Remove one of your schedules and its runs.",
            json!({ "schedule": schedule }),
            &["schedule"],
        ),
        tool(
            "list_saved_prompts",
            "Prompts people saved on this node, by name: every one, or a project's and the \
             machine's. Read only: they are a person's text.",
            json!({
                "project": { "type": "string", "description": "A project id (`list_projects`)." }
            }),
            &[],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn args(value: &Value) -> JsonObject {
        value.as_object().cloned().unwrap()
    }

    fn own() -> ScheduleTarget {
        ScheduleTarget::Session {
            session: vessel_proto::SessionId::random(),
        }
    }

    #[test]
    fn a_cadence_is_one_of_every_or_cron_with_its_zone() {
        let every = definition(&args(&json!({ "every_secs": 600, "prompt": "x" })), own()).unwrap();
        assert!(matches!(every.cadence, Cadence::Every { .. }));
        let cron = json!({ "cron": "0 9 * * 1-5", "tz": "Asia/Seoul", "prompt": "x" });
        assert!(matches!(
            definition(&args(&cron), own()).unwrap().cadence,
            Cadence::Cron { .. }
        ));
        for wrong in [
            json!({ "every_secs": 600, "cron": "@daily", "tz": "UTC", "prompt": "x" }),
            json!({ "prompt": "x" }),
            json!({ "cron": "@daily", "prompt": "x" }),
            json!({ "every_secs": 59, "prompt": "x" }),
            json!({ "every_secs": 600, "prompt": "x", "catch_up": "twice" }),
            json!({ "every_secs": 600, "prompt": "x", "max_runs": 0 }),
        ] {
            assert!(definition(&args(&wrong), own()).is_err(), "{wrong}");
        }
    }
}
