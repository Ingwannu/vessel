//! The store's side of schedules, saved prompts and hooks (protocol 11,
//! schema 12): what the node writes when one changes, and what it reads
//! back (`crate::automation` says how they are kept).

use chrono::{DateTime, Utc};
use vessel_proto::automation::{
    HookDelivery, HookInfo, RunNumber, SavedPrompt, ScheduleInfo, ScheduleRun,
};
use vessel_proto::{HookId, ProjectId, SavedPromptId, ScheduleId};

use super::Store;
use crate::automation::{self, AutomationWrite, StoredAutomation};
use crate::error::StoreError;
use crate::writer::Write;
use crate::{RunBindingStage, ScheduleRunBinding};

impl Store {
    /// Pin the run's exact future workspace and actor before either can be created.
    /// # Errors
    /// The run no longer exists, conflicts with another pin, or capacity/storage failed.
    pub async fn bind_schedule_run(&self, binding: ScheduleRunBinding) -> Result<(), StoreError> {
        self.automation_write(vec![AutomationWrite::BindRun(binding)])
            .await
    }

    /// Update an exact unsettled binding after actor registration or archive deferral.
    /// # Errors
    /// The pinned identity/stage changed or storage failed.
    pub async fn advance_schedule_run_binding(
        &self,
        previous: ScheduleRunBinding,
        stage: RunBindingStage,
    ) -> Result<(), StoreError> {
        self.automation_write(vec![AutomationWrite::AdvanceRunBinding { previous, stage }])
            .await
    }

    /// Release a pin once durable side-effect reconciliation is complete.
    /// # Errors
    /// The pinned identity/stage changed or storage failed.
    pub async fn finish_schedule_run_binding(
        &self,
        binding: ScheduleRunBinding,
    ) -> Result<(), StoreError> {
        self.automation_write(vec![AutomationWrite::FinishRunBinding(binding)])
            .await
    }

    /// Fetch the existing exact run pin without deriving ownership from a directory.
    /// # Errors
    /// The read or decoding failed.
    pub async fn schedule_run_binding(
        &self,
        schedule: ScheduleId,
        run: RunNumber,
    ) -> Result<Option<ScheduleRunBinding>, StoreError> {
        self.read(move |reader| {
            reader.with(|connection| automation::binding::find(connection, schedule, run))
        })
        .await
    }

    /// The bounded set of unresolved run pins, including those with deleted schedules.
    /// # Errors
    /// The read or decoding failed.
    pub async fn schedule_run_bindings(&self) -> Result<Vec<ScheduleRunBinding>, StoreError> {
        self.read(|reader| reader.with(automation::binding::load))
            .await
    }

    /// Whether the exact session has a Keep receipt. When requested is
    /// true, require its person's committed Archive mark as well.
    /// # Errors
    /// Storage or read failure.
    pub async fn schedule_keeps_session(
        &self,
        session: vessel_proto::SessionId,
        requested: bool,
    ) -> Result<bool, StoreError> {
        self.read(move |reader| {
            reader.with(|db| automation::binding::keeps_session(db, session, requested))
        })
        .await
    }

    /// Receipts ready for an archive retry or a person's deferred Keep release.
    /// # Errors
    /// Storage or decoding failure.
    pub async fn schedule_bindings_awaiting_archive(
        &self,
    ) -> Result<Vec<ScheduleRunBinding>, StoreError> {
        self.read(|reader| reader.with(automation::binding::awaiting_archive))
            .await
    }

    /// Schedules, hooks and the delivery keys hooks saw (protocol 11),
    /// read once when the node starts.
    ///
    /// # Errors
    /// The read failed.
    pub async fn automation(&self) -> Result<StoredAutomation, StoreError> {
        self.read(crate::reader::Reader::automation).await
    }

    /// Insert or replace a schedule, and with it one of its runs: a run
    /// about to be made is recorded in the same commit as the next due
    /// time it took, so a node stopped between the two never makes it
    /// twice (ADR 0033).
    ///
    /// # Errors
    /// The write failed.
    pub async fn put_schedule(
        &self,
        schedule: ScheduleInfo,
        run: Option<ScheduleRun>,
    ) -> Result<(), StoreError> {
        let id = schedule.id;
        let mut writes = vec![AutomationWrite::Schedule(Box::new(schedule))];
        if let Some(run) = run {
            writes.push(AutomationWrite::Run {
                schedule: id,
                run: Box::new(run),
            });
        }
        self.automation_write(writes).await
    }

    /// Insert or replace one run of a schedule (its outcome changed).
    ///
    /// # Errors
    /// The write failed (the schedule was deleted meanwhile, say).
    pub async fn put_run(&self, schedule: ScheduleId, run: ScheduleRun) -> Result<(), StoreError> {
        self.automation_write(vec![AutomationWrite::Run {
            schedule,
            run: Box::new(run),
        }])
        .await
    }

    /// Delete a schedule and its runs.
    ///
    /// # Errors
    /// The write failed.
    pub async fn forget_schedule(&self, schedule: ScheduleId) -> Result<(), StoreError> {
        self.automation_write(vec![AutomationWrite::ForgetSchedule(schedule)])
            .await
    }

    /// A schedule's newest runs, newest first (at most
    /// [`RUNS_KEPT`](crate::RUNS_KEPT)).
    ///
    /// # Errors
    /// The read failed.
    pub async fn runs(&self, schedule: ScheduleId) -> Result<Vec<ScheduleRun>, StoreError> {
        self.read(move |reader| reader.with(|connection| automation::runs(connection, schedule)))
            .await
    }

    /// Saved prompts by name (of one name, the machine's first): every one,
    /// or `project`'s and the machine's.
    ///
    /// # Errors
    /// The read failed.
    pub async fn saved_prompts(
        &self,
        project: Option<ProjectId>,
    ) -> Result<Vec<SavedPrompt>, StoreError> {
        self.read(move |reader| reader.with(|connection| automation::prompts(connection, project)))
            .await
    }

    /// One saved prompt.
    ///
    /// # Errors
    /// The read failed.
    pub async fn saved_prompt(&self, id: SavedPromptId) -> Result<Option<SavedPrompt>, StoreError> {
        self.read(move |reader| reader.with(|connection| automation::prompt(connection, id)))
            .await
    }

    /// The saved prompt called `name` in `project`'s scope (the machine's
    /// for `None`), and how many saved prompts the node keeps.
    ///
    /// # Errors
    /// The read failed.
    pub async fn prompt_named(
        &self,
        project: Option<ProjectId>,
        name: String,
    ) -> Result<(Option<SavedPromptId>, u64), StoreError> {
        self.read(move |reader| {
            reader.with(|connection| automation::named(connection, project, &name))
        })
        .await
    }

    /// Insert or replace a saved prompt. A name its scope already has
    /// fails the write (the node checks first).
    ///
    /// # Errors
    /// The write failed.
    pub async fn put_prompt(&self, prompt: SavedPrompt) -> Result<(), StoreError> {
        self.automation_write(vec![AutomationWrite::Prompt(Box::new(prompt))])
            .await
    }

    /// Delete a saved prompt.
    ///
    /// # Errors
    /// The write failed.
    pub async fn forget_prompt(&self, prompt: SavedPromptId) -> Result<(), StoreError> {
        self.automation_write(vec![AutomationWrite::ForgetPrompt(prompt)])
            .await
    }

    /// Insert or replace a hook.
    ///
    /// # Errors
    /// The write failed.
    pub async fn put_hook(&self, hook: HookInfo) -> Result<(), StoreError> {
        self.automation_write(vec![AutomationWrite::Hook(Box::new(hook))])
            .await
    }

    /// Delete a hook, its log and its keys.
    ///
    /// # Errors
    /// The write failed.
    pub async fn forget_hook(&self, hook: HookId) -> Result<(), StoreError> {
        self.automation_write(vec![AutomationWrite::ForgetHook(hook)])
            .await
    }

    /// Add a delivery to a hook's log (trimmed to the newest
    /// [`DELIVERIES_KEPT`](crate::DELIVERIES_KEPT)).
    ///
    /// # Errors
    /// The write failed.
    pub async fn put_delivery(
        &self,
        hook: HookId,
        delivery: HookDelivery,
    ) -> Result<(), StoreError> {
        self.automation_write(vec![AutomationWrite::Delivery {
            hook,
            delivery: Box::new(delivery),
        }])
        .await
    }

    /// A hook's newest deliveries, newest first.
    ///
    /// # Errors
    /// The read failed.
    pub async fn deliveries(&self, hook: HookId) -> Result<Vec<HookDelivery>, StoreError> {
        self.read(move |reader| reader.with(|connection| automation::deliveries(connection, hook)))
            .await
    }

    /// Remember a delivery key a hook saw at `at`, and forget its keys
    /// seen before `forget_before`.
    ///
    /// # Errors
    /// The write failed.
    pub async fn remember_key(
        &self,
        hook: HookId,
        key: String,
        at: DateTime<Utc>,
        forget_before: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        self.automation_write(vec![AutomationWrite::Key {
            hook,
            key,
            at,
            forget_before,
        }])
        .await
    }

    async fn automation_write(&self, writes: Vec<AutomationWrite>) -> Result<(), StoreError> {
        self.write(|done| Write::Automation { writes, done }).await
    }
}
