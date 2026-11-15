//! Staged A2 resources for W12's sole bundle. No live registration or globals.
//! Snapshot readiness cannot create transport, operation or tab-close authority.
mod state;
#[cfg(test)]
mod tests;
pub(crate) use state::LifecycleResources;

use crate::areas::layout::residents::{SessionAdmission, SessionResources};
use vessel_app_view::residents::{ResidentProblem, ResidentToken};
use vessel_proto::{OptionChoice, SessionInfo};

/// Domain values only; actual fields, selection/undo and forms stay in the bundle.
#[derive(Debug)]
pub(crate) struct Snapshot {
    resident: ResidentToken,
    info: Box<SessionInfo>,
    choices: Vec<OptionChoice>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Problem {
    Scope(ResidentProblem),
    Loading,
    WrongResource,
    Held,
    Exhausted,
}
impl Snapshot {
    /// Invoke only inside canonical receive after original delivery validity.
    /// The original minter receipt binds identity; no legacy Open binding or
    /// selected-global lookup may supply missing origin or refresh visibility.
    pub(crate) fn received(
        resources: &SessionResources,
        admission: &SessionAdmission,
    ) -> Result<Self, Problem> {
        let resident = admission.binding.resident;
        if admission.context.resident != resident
            || admission.connection.node() != resident.address.node
        {
            return Err(Problem::WrongResource);
        }
        let open = resources.session.open.as_ref().ok_or(Problem::Loading)?;
        let shown = resources
            .conversation
            .shown
            .as_ref()
            .ok_or(Problem::Loading)?;
        let info = resources
            .conversation
            .info
            .as_ref()
            .ok_or(Problem::Loading)?;
        if admission.connection.machine().as_str() != open.machine
            || open.session != resident.address.session
            || shown.machine != open.machine
            || shown.session != open.session
            || info.id != open.session
        {
            return Err(Problem::WrongResource);
        }
        let choices = resources.conversation.timeline.options().map_or_else(
            || info.spec.mode.options().to_vec(),
            |options| {
                options
                    .iter()
                    .filter_map(vessel_proto::SessionOption::choice)
                    .collect()
            },
        );
        Ok(Self {
            resident,
            info: info.clone(),
            choices,
        })
    }
    pub(crate) const fn resident(&self) -> ResidentToken {
        self.resident
    }
    pub(crate) fn info(&self) -> &SessionInfo {
        &self.info
    }
    pub(crate) fn choices(&self) -> &[OptionChoice] {
        &self.choices
    }
}
