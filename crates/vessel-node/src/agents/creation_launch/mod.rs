//! Concrete staged creation owner; no RPC, permission token or spawn is added.
//! The driver keeps AgentReservation independently of the reply/capsule.
use super::{
    AgentReservation, CapturedParentExecution, CreationCountCorrelation, ExecutionObservation,
};
use crate::sessions::CreationPlacement;
use std::{
    fmt,
    time::{Duration, Instant},
};
use vessel_proto::creation::{Receipt, Resource, Stage};
use vessel_proto::{Creator, SessionId, SessionMode, SessionSpec, WorkspaceId};
use vessel_session::LaunchOptions;

/// A candidate cannot wait indefinitely between preparation and guarded insert.
const MAX_LAUNCH_ADMISSION_AGE: Duration = Duration::from_secs(5);

pub(crate) struct LaunchInput {
    parent: CapturedParentExecution,
    placement: CreationPlacement,
    spec: SessionSpec,
    options: LaunchOptions,
}

impl LaunchInput {
    pub(crate) fn new(
        parent: CapturedParentExecution,
        placement: CreationPlacement,
        spec: SessionSpec,
        options: LaunchOptions,
    ) -> Self {
        Self {
            parent,
            placement,
            spec,
            options,
        }
    }
}

pub(crate) struct Candidate<'c> {
    data: Box<Data<'c>>,
}
pub(crate) struct TrackedLaunch<'c> {
    data: Box<Data<'c>>,
}
struct Data<'c> {
    input: LaunchInput,
    count: CreationCountCorrelation<'c>,
    expires: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CandidateProblem {
    Expired,
    Binding,
}

pub(crate) struct PreparationRejected {
    pub(crate) problem: CandidateProblem,
    pub(crate) input: Box<LaunchInput>,
}

impl fmt::Debug for LaunchInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CreationLaunchInput")
            .field("child", &self.placement.child())
            .field("workspace", &self.placement.workspace())
            .finish_non_exhaustive()
    }
}
impl fmt::Debug for PreparationRejected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparationRejected")
            .field("problem", &self.problem)
            .finish_non_exhaustive()
    }
}
impl fmt::Debug for Candidate<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.input.fmt(f)
    }
}
impl fmt::Debug for TrackedLaunch<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.input.fmt(f)
    }
}

impl<'c> Candidate<'c> {
    /// All command preparation/allocation happens before the Agents guard.
    /// Failure returns the concrete input; it never releases a pending start.
    pub(crate) fn prepare(
        input: LaunchInput,
        count: &mut AgentReservation<'c>,
        begin: &Receipt,
    ) -> Result<Self, PreparationRejected> {
        let reserved = &begin.snapshot().reserved;
        let placement = &input.placement;
        if begin.in_flight() != Some(Stage::Resource)
            || reserved.workspace != placement.workspace()
            || reserved.resource
                != Some(Resource::Agent {
                    session: placement.child(),
                })
            || input.parent.stamp().session != placement.parent()
            || input.options.id != Some(placement.child())
            || !matches!(input.spec.mode, SessionMode::Structured { .. })
            || input.spec.place.is_some()
            || input.options.via.is_some()
            || input.options.created_by
                != (Creator::Agent {
                    node: input.parent.stamp().node,
                    session: placement.parent(),
                })
            || input.spec.cwd.as_deref() != Some(placement.root().path())
            || placement.start().workspace() != placement.workspace()
            || placement.start().resource()
                != (vessel_proto::registry::WorkspaceResource::Session {
                    session: placement.child(),
                })
        {
            return Err(PreparationRejected {
                problem: CandidateProblem::Binding,
                input: Box::new(input),
            });
        }
        let Some(expires) = Instant::now().checked_add(MAX_LAUNCH_ADMISSION_AGE) else {
            return Err(PreparationRejected {
                problem: CandidateProblem::Expired,
                input: Box::new(input),
            });
        };
        let Ok(correlation) = count.creation_correlation() else {
            return Err(PreparationRejected {
                problem: CandidateProblem::Binding,
                input: Box::new(input),
            });
        };
        if correlation.child() != placement.child()
            || correlation.begin() != (reserved.operation, begin.snapshot().revision)
        {
            return Err(PreparationRejected {
                problem: CandidateProblem::Binding,
                input: Box::new(input),
            });
        }
        Ok(Self {
            data: Box::new(Data {
                input,
                count: correlation,
                expires,
            }),
        })
    }
    pub(in crate::agents) fn owner_matches(&self, registry: &super::Agents) -> bool {
        self.data.input.placement.owner_matches(registry)
    }
    pub(in crate::agents) fn capture(&self) -> &CapturedParentExecution {
        &self.data.input.parent
    }
    pub(in crate::agents) fn count(&self) -> &CreationCountCorrelation<'c> {
        &self.data.count
    }
    pub(in crate::agents) fn check_deadline(&self) -> Result<(), CandidateProblem> {
        if Instant::now() >= self.data.expires {
            Err(CandidateProblem::Expired)
        } else {
            Ok(())
        }
    }
    pub(in crate::agents) fn child(&self) -> SessionId {
        self.data.input.placement.child()
    }
    pub(in crate::agents) fn workspace(&self) -> WorkspaceId {
        self.data.input.placement.workspace()
    }
    /// ONLY the common same-guard current-parent/count check may call this.
    /// It moves the already allocated concrete owner: no lock, callback, I/O,
    /// recapture, spawn or allocation occurs in this insertion.
    pub(in crate::agents) fn insert(self) -> TrackedLaunch<'c> {
        TrackedLaunch { data: self.data }
    }
}

impl TrackedLaunch<'_> {
    /// Read-only observation outside the registry guard; never fresh permission.
    pub(crate) fn observation(&self) -> ExecutionObservation {
        self.data.input.parent.observation()
    }
    /// Pending spawn ownership must survive this notification through settlement.
    pub(crate) async fn invalidated(&mut self) -> ExecutionObservation {
        self.data.input.parent.invalidated().await
    }
}
