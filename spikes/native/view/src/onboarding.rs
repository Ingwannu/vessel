//! One client-local setup decision per launch, with visit-safe asynchronous effects.
//!
//! Completion is independent of node settings. A late project/config result must
//! never advance a visit that the person has already left.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::devices::MachineKey;

pub mod orchestration;

/// Setup steps in the reference order; existing screens perform their effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Theme changes apply immediately.
    Appearance,
    /// Install and authenticate existing providers.
    Providers,
    /// Register projects on the selected host.
    Projects,
    /// Apply offered tool, skill and CLI choices on Continue.
    Orchestration,
    /// Pair another device.
    Phone,
}

const WIDE_STEPS: &[Step] = &[
    Step::Appearance,
    Step::Providers,
    Step::Projects,
    Step::Orchestration,
    Step::Phone,
];
const COMPACT_STEPS: &[Step] = &[
    Step::Appearance,
    Step::Providers,
    Step::Projects,
    Step::Orchestration,
];

/// Form factor is fixed for one opening, so a resize cannot invalidate its rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// Includes pairing another device.
    Wide,
    /// Omits Phone as Paseo's compact setup does.
    Compact,
}

impl Layout {
    /// The exact ordered step set for one opening.
    #[must_use]
    pub const fn steps(self) -> &'static [Step] {
        match self {
            Self::Wide => WIDE_STEPS,
            Self::Compact => COMPACT_STEPS,
        }
    }
}

/// Storage read outcome; inaccessible storage must not trap existing users.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Completion {
    /// Read has not finished.
    Loading,
    /// No valid completion record exists.
    Pending,
    /// Completed or the record could not be read.
    Completed,
}

/// Only completion is persisted: unfinished setup starts again at Appearance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionRecord {
    /// Time this client finished, skipped or closed setup.
    pub completed_at: DateTime<Utc>,
}

/// Facts that must all be known before the once-per-launch decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupHost {
    /// Missing, offline or directory still loading.
    Waiting,
    /// Connected host whose hydrated directory has no project.
    Empty(MachineKey),
    /// Connected host already has projects.
    Existing,
}

/// Identity of this stay on a step, retained by every asynchronous operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visit(u64);

/// A visible walkthrough, scoped to the host chosen at its opening.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Open {
    host: MachineKey,
    layout: Layout,
    step: Step,
    visit: Visit,
}

impl Open {
    /// Stable host of this flow, regardless of settings selection afterwards.
    #[must_use]
    pub const fn host(&self) -> &MachineKey {
        &self.host
    }
    /// Ordered rail, fixed at opening.
    #[must_use]
    pub const fn steps(&self) -> &'static [Step] {
        self.layout.steps()
    }
    /// Current step.
    #[must_use]
    pub const fn step(&self) -> Step {
        self.step
    }
    /// Capture before starting project/config work.
    #[must_use]
    pub const fn visit(&self) -> Visit {
        self.visit
    }
}

/// Visibility never changes automatically after the initial decision.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum State {
    /// Completion or selected host directory is still loading.
    #[default]
    Deciding,
    /// Finished, skipped, or an existing user's client.
    Hidden,
    /// Setup is visible.
    Open(Open),
}

/// A transition's persistence requirement cannot be silently lost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum Effect {
    /// No completion write is needed.
    None,
    /// Persist completion, including Skip and close.
    Complete,
}

/// Checked visit exhaustion refuses navigation rather than reusing an old visit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("setup visit counter exhausted")]
pub struct VisitExhausted;

/// Owns the monotonic visit source for the lifetime of this launch.
#[derive(Debug, Default)]
pub struct Model {
    state: State,
    visits: u64,
}

impl Model {
    /// Current visibility; views never recreate the machine when they redraw.
    #[must_use]
    pub const fn state(&self) -> &State {
        &self.state
    }

    /// Decide only after completion and host readiness are known.
    ///
    /// # Errors
    /// Visit identity space is exhausted.
    pub fn decide(
        &mut self,
        completion: Completion,
        host: SetupHost,
        layout: Layout,
    ) -> Result<(), VisitExhausted> {
        if self.state != State::Deciding {
            return Ok(());
        }
        match completion {
            Completion::Loading => Ok(()),
            Completion::Completed => {
                self.state = State::Hidden;
                Ok(())
            }
            Completion::Pending => match host {
                SetupHost::Waiting => Ok(()),
                SetupHost::Existing => {
                    self.state = State::Hidden;
                    Ok(())
                }
                SetupHost::Empty(host) => self.reopen(host, layout),
            },
        }
    }

    /// Explicit Settings action can reopen even a completed/existing client.
    ///
    /// # Errors
    /// Visit identity space is exhausted.
    pub fn reopen(&mut self, host: MachineKey, layout: Layout) -> Result<(), VisitExhausted> {
        let visit = self.next_visit()?;
        self.state = State::Open(Open {
            host,
            layout,
            step: Step::Appearance,
            visit,
        });
        Ok(())
    }

    /// Advance only the visit whose effects have all succeeded.
    ///
    /// # Errors
    /// Visit identity space is exhausted.
    pub fn next(&mut self, visit: Visit) -> Result<Effect, VisitExhausted> {
        let State::Open(open) = &self.state else {
            return Ok(Effect::None);
        };
        if open.visit != visit {
            return Ok(Effect::None);
        }
        let next = open
            .steps()
            .iter()
            .skip_while(|step| **step != open.step)
            .nth(1)
            .copied();
        match next {
            Some(step) => {
                self.go_to(step)?;
                Ok(Effect::None)
            }
            None => Ok(self.complete()),
        }
    }

    /// Back on the first step stays there and retains its visit.
    ///
    /// # Errors
    /// Visit identity space is exhausted.
    pub fn back(&mut self) -> Result<(), VisitExhausted> {
        let State::Open(open) = &self.state else {
            return Ok(());
        };
        let step = open
            .steps()
            .iter()
            .take_while(|step| **step != open.step)
            .last()
            .copied();
        if let Some(step) = step {
            self.go_to(step)?;
        }
        Ok(())
    }

    /// The rail can visit any offered step; excluded/current steps do nothing.
    ///
    /// # Errors
    /// Visit identity space is exhausted.
    pub fn go_to(&mut self, step: Step) -> Result<(), VisitExhausted> {
        let State::Open(open) = &self.state else {
            return Ok(());
        };
        if open.step == step || !open.steps().contains(&step) {
            return Ok(());
        }
        let visit = self.next_visit()?;
        if let State::Open(open) = &mut self.state {
            open.step = step;
            open.visit = visit;
        }
        Ok(())
    }

    /// Hide immediately; earlier successful step effects are never rolled back.
    pub fn complete(&mut self) -> Effect {
        self.state = State::Hidden;
        Effect::Complete
    }

    fn next_visit(&mut self) -> Result<Visit, VisitExhausted> {
        self.visits = self.visits.checked_add(1).ok_or(VisitExhausted)?;
        Ok(Visit(self.visits))
    }
}

#[cfg(test)]
mod tests;
