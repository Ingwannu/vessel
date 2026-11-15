//! An agent's plan as Paseo's task list draws it (`types/stream.ts`,
//! `deriveTaskActivities`; `components/message.tsx`, `TodoListCard`): a
//! row for what changed in it ("Created 4 tasks", "Started", "Completed"
//! with the task), opening to the plan as it stood.
//!
//! [Decision Log]
//! - Purpose: an agent sends its whole plan again at every change (ACP's
//!   `plan` update); the timeline drew one quiet "Plan · 1 of 4 done" line
//!   per update, so a person could not tell what had changed.
//! - The reference: Paseo compares each plan with the one before by task
//!   (its id, else its place and its words) and says what changed: the
//!   first plan "Created N tasks"; a task new to it "Added"; a task that
//!   went under way "Started"; one done "Completed". An update that
//!   changes nothing a person reads only refreshes the last row's plan,
//!   and a task added to a list just created, all still to do, is part of
//!   the creation.
//! - Choice: the same, in the fold ([`plan_activities`] here; the rows in
//!   `agent.rs`). ACP's plan steps carry no id, so a step is known by its
//!   place and its words, as Paseo knows a task without one.
//! - Cost: a step reworded or moved reads as a new one ("Added"), as in
//!   Paseo.

use vessel_proto::{PlanEntry, PlanStatus};

/// What changed in a plan, as its row says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanActivity {
    /// The first plan: how many steps it has.
    Created {
        /// Its steps.
        count: usize,
    },
    /// A step new to the plan.
    Added(String),
    /// A step under way.
    Started(String),
    /// A step done.
    Completed(String),
}

impl PlanActivity {
    /// The step it names, when it names one.
    #[must_use]
    pub fn step(&self) -> Option<&str> {
        match self {
            Self::Created { .. } => None,
            Self::Added(step) | Self::Started(step) | Self::Completed(step) => Some(step),
        }
    }
}

/// What changed from `previous` to `current`, in the plan's order: the
/// first plan is created; then each step new to it is added, and each
/// step whose state moved to under way or done is started or completed.
#[must_use]
pub fn plan_activities(previous: &[PlanEntry], current: &[PlanEntry]) -> Vec<PlanActivity> {
    if previous.is_empty() {
        return if current.is_empty() {
            Vec::new()
        } else {
            vec![PlanActivity::Created {
                count: current.len(),
            }]
        };
    }
    current
        .iter()
        .enumerate()
        .filter_map(|(at, step)| {
            let before = previous
                .get(at)
                .filter(|prior| prior.content == step.content);
            let Some(before) = before else {
                return Some(PlanActivity::Added(step.content.clone()));
            };
            if before.status == step.status {
                return None;
            }
            match step.status {
                PlanStatus::Completed => Some(PlanActivity::Completed(step.content.clone())),
                PlanStatus::InProgress => Some(PlanActivity::Started(step.content.clone())),
                PlanStatus::Pending => None,
            }
        })
        .collect()
}

/// Whether every step of `plan` is still to do.
#[must_use]
pub fn all_pending(plan: &[PlanEntry]) -> bool {
    plan.iter().all(|step| step.status == PlanStatus::Pending)
}
