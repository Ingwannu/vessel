//! Schedules, their runs, saved prompts and hooks (protocol 11, schema 13,
//! ADRs 0033 and 0034): the node's record of what it does on its own.
//!
//! [Decision Log]
//! - Purpose: a schedule's next due time, its count of runs and its state
//!   must survive a restart (a node that forgot a schedule had run would
//!   run it again; one that forgot its count would pass `max_runs`), and
//!   so must its newest runs, the saved prompts, the hooks, their logs and
//!   the delivery keys seen in the last day.
//! - Choice: workspace's way (schema 9): each schedule, saved prompt and
//!   hook is one row holding its `vessel-proto` type as JSON, written whole
//!   on every change, so a field added to the wire needs no migration. A
//!   run and a delivery are rows of their own under their schedule or hook
//!   (deleted with it), trimmed to the newest [`RUNS_KEPT`] and
//!   [`DELIVERIES_KEPT`] when one is added. A saved prompt's name is unique
//!   within its scope (its project, or `''` for the machine's), which the
//!   database holds too.
//! - The node keeps schedules and hooks in memory and reads them once at
//!   start; runs, deliveries and saved prompts are read when asked for.
//! - Rows that no longer decode are skipped with a warning: a bad row
//!   loses that one record, not the node's start.
//! - Hook secrets are not here: they are files of their own (ADR 0034), so
//!   a copy of the database signs nothing.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use vessel_proto::automation::{HookDelivery, HookInfo, SavedPrompt, ScheduleInfo, ScheduleRun};
use vessel_proto::{HookId, ProjectId, SavedPromptId, ScheduleId};

use crate::codec::{encode, time_to_sql};
use crate::error::StoreError;

/// Runs kept per schedule.
pub const RUNS_KEPT: u32 = 50;
/// Deliveries kept per hook.
pub const DELIVERIES_KEPT: u32 = 100;

/// What the store holds about schedules and hooks, read once at start.
#[derive(Debug, Clone, Default)]
pub struct StoredAutomation {
    /// Every schedule, oldest first.
    pub schedules: Vec<ScheduleInfo>,
    /// Every hook, oldest first.
    pub hooks: Vec<HookInfo>,
    /// The delivery keys each hook saw, newest last.
    pub keys: Vec<(HookId, String, DateTime<Utc>)>,
}

/// A change to that record.
#[derive(Debug)]
pub(crate) enum AutomationWrite {
    /// Insert or replace a schedule.
    Schedule(Box<ScheduleInfo>),
    /// Delete a schedule and its runs.
    ForgetSchedule(ScheduleId),
    /// Insert or replace a run, then trim the schedule's runs.
    Run {
        schedule: ScheduleId,
        run: Box<ScheduleRun>,
    },
    /// Insert or replace a saved prompt.
    Prompt(Box<SavedPrompt>),
    /// Delete a saved prompt.
    ForgetPrompt(SavedPromptId),
    /// Insert or replace a hook.
    Hook(Box<HookInfo>),
    /// Delete a hook, its log and its keys.
    ForgetHook(HookId),
    /// Add a delivery to a hook's log, then trim it.
    Delivery {
        hook: HookId,
        delivery: Box<HookDelivery>,
    },
    /// Remember a delivery key, and forget those seen before `forget_before`.
    Key {
        hook: HookId,
        key: String,
        at: DateTime<Utc>,
        forget_before: DateTime<Utc>,
    },
}

/// A saved prompt's scope: its project, or the machine's own.
fn scope(project: Option<ProjectId>) -> String {
    project
        .map(|project| project.to_string())
        .unwrap_or_default()
}

/// Apply one change. Writer side.
pub(crate) fn apply(connection: &Connection, write: &AutomationWrite) -> Result<(), StoreError> {
    let changed = match write {
        AutomationWrite::Schedule(schedule) => connection
            .prepare_cached(
                "INSERT INTO schedules (id, created_at, info) VALUES (?1, ?2, ?3)
                 ON CONFLICT (id) DO UPDATE SET info = excluded.info",
            )?
            .execute(params![
                schedule.id.to_string(),
                time_to_sql(schedule.created_at)?,
                encode(schedule)?
            ])?,
        AutomationWrite::ForgetSchedule(id) => connection
            .prepare_cached("DELETE FROM schedules WHERE id = ?1")?
            .execute([id.to_string()])?,
        AutomationWrite::Run { schedule, run } => {
            let number = run.number.get();
            let written = connection
                .prepare_cached(
                    "INSERT INTO schedule_runs (schedule, number, run) VALUES (?1, ?2, ?3)
                     ON CONFLICT (schedule, number) DO UPDATE SET run = excluded.run",
                )?
                .execute(params![schedule.to_string(), number, encode(run)?])?;
            let _trimmed = connection
                .prepare_cached("DELETE FROM schedule_runs WHERE schedule = ?1 AND number <= ?2")?
                .execute(params![
                    schedule.to_string(),
                    number.saturating_sub(RUNS_KEPT)
                ])?;
            written
        }
        AutomationWrite::Prompt(prompt) => connection
            .prepare_cached(
                "INSERT INTO saved_prompts (id, scope, name, created_at, info)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (id) DO UPDATE SET name = excluded.name, info = excluded.info",
            )?
            .execute(params![
                prompt.id.to_string(),
                scope(prompt.project),
                prompt.name.as_str(),
                time_to_sql(prompt.created_at)?,
                encode(prompt)?
            ])?,
        AutomationWrite::ForgetPrompt(id) => connection
            .prepare_cached("DELETE FROM saved_prompts WHERE id = ?1")?
            .execute([id.to_string()])?,
        AutomationWrite::Hook(hook) => connection
            .prepare_cached(
                "INSERT INTO hooks (id, created_at, info) VALUES (?1, ?2, ?3)
                 ON CONFLICT (id) DO UPDATE SET info = excluded.info",
            )?
            .execute(params![
                hook.id.to_string(),
                time_to_sql(hook.created_at)?,
                encode(hook)?
            ])?,
        AutomationWrite::ForgetHook(id) => connection
            .prepare_cached("DELETE FROM hooks WHERE id = ?1")?
            .execute([id.to_string()])?,
        AutomationWrite::Delivery { hook, delivery } => {
            let written = connection
                .prepare_cached(
                    "INSERT INTO hook_deliveries (hook, seq, delivery)
                     VALUES (?1, (SELECT COALESCE(MAX(seq), 0) + 1 FROM hook_deliveries
                                  WHERE hook = ?1), ?2)",
                )?
                .execute(params![hook.to_string(), encode(delivery)?])?;
            let _trimmed = connection
                .prepare_cached(
                    "DELETE FROM hook_deliveries WHERE hook = ?1 AND seq <=
                     (SELECT MAX(seq) FROM hook_deliveries WHERE hook = ?1) - ?2",
                )?
                .execute(params![hook.to_string(), DELIVERIES_KEPT])?;
            written
        }
        AutomationWrite::Key {
            hook,
            key,
            at,
            forget_before,
        } => {
            let _forgotten = connection
                .prepare_cached("DELETE FROM hook_keys WHERE hook = ?1 AND seen_at < ?2")?
                .execute(params![hook.to_string(), time_to_sql(*forget_before)?])?;
            connection
                .prepare_cached(
                    "INSERT OR REPLACE INTO hook_keys (hook, key, seen_at) VALUES (?1, ?2, ?3)",
                )?
                .execute(params![hook.to_string(), key, time_to_sql(*at)?])?
        }
    };
    tracing::trace!(rows = changed, "automation record written");
    Ok(())
}

/// Each row's JSON, decoded; one that does not decode is skipped.
fn decoded<T: serde::de::DeserializeOwned>(
    connection: &Connection,
    sql: &str,
    params: impl rusqlite::Params,
    what: &str,
) -> Result<Vec<T>, StoreError> {
    let mut statement = connection.prepare_cached(sql)?;
    let mut rows = statement.query(params)?;
    let mut found = Vec::new();
    while let Some(row) = rows.next()? {
        let text: String = row.get(0)?;
        match serde_json::from_str(&text) {
            Ok(value) => found.push(value),
            Err(error) => tracing::warn!(%error, what, "a stored record does not decode; skipped"),
        }
    }
    Ok(found)
}

/// Schedules, hooks and keys, oldest first. Reader side.
pub(crate) fn load(connection: &Connection) -> Result<StoredAutomation, StoreError> {
    let schedules = decoded(
        connection,
        "SELECT info FROM schedules ORDER BY created_at, id",
        [],
        "schedule",
    )?;
    let hooks = decoded(
        connection,
        "SELECT info FROM hooks ORDER BY created_at, id",
        [],
        "hook",
    )?;
    let mut keys = Vec::new();
    let mut statement =
        connection.prepare_cached("SELECT hook, key, seen_at FROM hook_keys ORDER BY seen_at")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let hook: String = row.get(0)?;
        let key: String = row.get(1)?;
        let seen: i64 = row.get(2)?;
        match hook.parse() {
            Ok(hook) => keys.push((hook, key, crate::codec::time_from_sql(seen))),
            Err(_bad) => tracing::warn!("a stored delivery key's hook does not decode; skipped"),
        }
    }
    Ok(StoredAutomation {
        schedules,
        hooks,
        keys,
    })
}

/// A schedule's runs, newest first. Reader side.
pub(crate) fn runs(
    connection: &Connection,
    schedule: ScheduleId,
) -> Result<Vec<ScheduleRun>, StoreError> {
    decoded(
        connection,
        "SELECT run FROM schedule_runs WHERE schedule = ?1 ORDER BY number DESC LIMIT ?2",
        params![schedule.to_string(), RUNS_KEPT],
        "run",
    )
}

/// A hook's deliveries, newest first. Reader side.
pub(crate) fn deliveries(
    connection: &Connection,
    hook: HookId,
) -> Result<Vec<HookDelivery>, StoreError> {
    decoded(
        connection,
        "SELECT delivery FROM hook_deliveries WHERE hook = ?1 ORDER BY seq DESC LIMIT ?2",
        params![hook.to_string(), DELIVERIES_KEPT],
        "delivery",
    )
}

/// Saved prompts by name: every one, or `project`'s and the machine's.
/// Reader side.
pub(crate) fn prompts(
    connection: &Connection,
    project: Option<ProjectId>,
) -> Result<Vec<SavedPrompt>, StoreError> {
    match project {
        None => decoded(
            connection,
            "SELECT info FROM saved_prompts ORDER BY name, id",
            [],
            "saved prompt",
        ),
        Some(project) => decoded(
            connection,
            "SELECT info FROM saved_prompts WHERE scope IN ('', ?1) ORDER BY name, id",
            [project.to_string()],
            "saved prompt",
        ),
    }
}

/// One saved prompt. Reader side.
pub(crate) fn prompt(
    connection: &Connection,
    id: SavedPromptId,
) -> Result<Option<SavedPrompt>, StoreError> {
    Ok(decoded(
        connection,
        "SELECT info FROM saved_prompts WHERE id = ?1",
        [id.to_string()],
        "saved prompt",
    )?
    .into_iter()
    .next())
}

/// The saved prompt called `name` in `project`'s scope (or the machine's),
/// and how many saved prompts there are. Reader side.
pub(crate) fn named(
    connection: &Connection,
    project: Option<ProjectId>,
    name: &str,
) -> Result<(Option<SavedPromptId>, u64), StoreError> {
    let found: Option<String> = connection
        .prepare_cached("SELECT id FROM saved_prompts WHERE scope = ?1 AND name = ?2")?
        .query_row(params![scope(project), name], |row| row.get(0))
        .optional()?;
    let count: i64 = connection
        .prepare_cached("SELECT COUNT(*) FROM saved_prompts")?
        .query_row([], |row| row.get(0))?;
    Ok((
        found.and_then(|id| id.parse().ok()),
        u64::try_from(count).unwrap_or(0),
    ))
}
