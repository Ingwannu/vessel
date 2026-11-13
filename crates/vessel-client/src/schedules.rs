//! Protocol 11's schedule calls (milestone 11, track D, ADR 0033), for the
//! CLI and the agent tools. Each is [`NodeClient::call`] with its request.

use vessel_proto::ScheduleId;
use vessel_proto::automation::{
    CreateSchedule, DeleteSchedule, GetSchedule, ListRuns, ListSchedules, PauseSchedule,
    ResumeSchedule, RunSchedule, ScheduleDefinition, ScheduleInfo, ScheduleRun, UpdateSchedule,
};
use vessel_rpc::CallError;

use crate::NodeClient;

impl NodeClient {
    /// `schedule.list`: every schedule on the node, newest first.
    ///
    /// # Errors
    /// `NotInProtocol` before protocol 11; otherwise see [`CallError`].
    pub async fn schedules(&self) -> Result<Vec<ScheduleInfo>, CallError> {
        self.call(ListSchedules {}).await.map(|list| list.schedules)
    }

    /// `schedule.get`.
    ///
    /// # Errors
    /// `not_found`; otherwise see [`CallError`].
    pub async fn schedule(&self, schedule: ScheduleId) -> Result<ScheduleInfo, CallError> {
        self.call(GetSchedule { schedule }).await
    }

    /// `schedule.create`: make a schedule, paused from the start if asked.
    ///
    /// # Errors
    /// `invalid_request` (an unknown zone, a cron that never runs, an end
    /// before the first due time, a terminal or resuming spec),
    /// `not_found`, `unsupported`, `forbidden`, `conflict` (the node's
    /// bound); otherwise see [`CallError`].
    pub async fn create_schedule(
        &self,
        definition: ScheduleDefinition,
        paused: bool,
    ) -> Result<ScheduleInfo, CallError> {
        self.call(CreateSchedule { definition, paused }).await
    }

    /// `schedule.update`: replace the definition made from `revision`.
    ///
    /// # Errors
    /// `conflict` for another revision or an ended schedule; as
    /// [`create_schedule`](Self::create_schedule).
    pub async fn update_schedule(
        &self,
        schedule: ScheduleId,
        revision: u32,
        definition: ScheduleDefinition,
    ) -> Result<ScheduleInfo, CallError> {
        self.call(UpdateSchedule {
            schedule,
            revision,
            definition,
        })
        .await
    }

    /// `schedule.pause`.
    ///
    /// # Errors
    /// `conflict` for an ended schedule, `forbidden`, `not_found`.
    pub async fn pause_schedule(&self, schedule: ScheduleId) -> Result<ScheduleInfo, CallError> {
        self.call(PauseSchedule { schedule }).await
    }

    /// `schedule.resume`: from now.
    ///
    /// # Errors
    /// As [`pause_schedule`](Self::pause_schedule).
    pub async fn resume_schedule(&self, schedule: ScheduleId) -> Result<ScheduleInfo, CallError> {
        self.call(ResumeSchedule { schedule }).await
    }

    /// `schedule.run`: once now, answered with the run's outcome.
    ///
    /// # Errors
    /// `conflict` while a run of it is being made or once it has ended,
    /// `forbidden` (agents), `unavailable` when no run slot came free.
    pub async fn run_schedule(&self, schedule: ScheduleId) -> Result<ScheduleRun, CallError> {
        self.call(RunSchedule { schedule }).await
    }

    /// `schedule.delete`.
    ///
    /// # Errors
    /// `forbidden`, `not_found`.
    pub async fn delete_schedule(&self, schedule: ScheduleId) -> Result<(), CallError> {
        self.call(DeleteSchedule { schedule }).await.map(|_ack| ())
    }

    /// `schedule.runs`: the newest 50, newest first.
    ///
    /// # Errors
    /// `not_found`.
    pub async fn schedule_runs(&self, schedule: ScheduleId) -> Result<Vec<ScheduleRun>, CallError> {
        self.call(ListRuns { schedule }).await.map(|list| list.runs)
    }
}
