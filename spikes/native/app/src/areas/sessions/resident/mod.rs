//! Staged A2 resources for W12's sole bundle. No live registration or globals.
//! Snapshot readiness cannot create transport, operation or tab-close authority.
mod state;
#[cfg(test)]
mod tests;
pub(crate) use state::LifecycleResources;

use crate::areas::layout::residents::SessionResidents;
use vessel_app_view::residents::{ResidentProblem, ResidentToken, SessionContext};
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
    /// Borrow only through the canonical getter. There is no selected fallback.
    pub(crate) fn capture(
        residents: &SessionResidents,
        context: &SessionContext,
    ) -> Result<Self, Problem> {
        let resources = residents.resources(context).map_err(Problem::Scope)?;
        let open = resources.session.open.as_ref().ok_or(Problem::Loading)?;
        let binding = open.binding.as_ref().ok_or(Problem::Loading)?;
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
        if binding.address != context.resident.address
            || binding.host.as_str() != open.machine
            || open.session != context.resident.address.session
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
            resident: context.resident,
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
