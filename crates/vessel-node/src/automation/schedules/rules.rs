//! Who may do what to a schedule (ADR 0033, the contract's "Who a run acts
//! as, and who may change what"). Pure: the scheduler asks before it
//! changes anything, and the tests ask without one.

use vessel_proto::Creator;

use crate::automation::error::AutomationError;

/// Schedules an agent's session may have at once.
pub(crate) const AGENT_SCHEDULES: usize = 8;
/// Schedules agents may have on a node at once.
pub(crate) const AGENT_SCHEDULES_ON_NODE: usize = 64;
/// Schedules people may have on a node, in any state.
pub(crate) const PEOPLE_SCHEDULES: usize = 256;

/// Who asks, as the rules need to know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Asker {
    /// Who they are on this node (what `defined_by` records).
    pub(crate) by: Creator,
    /// How they reach it.
    pub(crate) via: Via,
}

/// How an asker reaches the node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Via {
    /// The owner or a paired device, directly.
    Person,
    /// A session's agent of this node, through its credential.
    Agent,
    /// A call another node passed on (`fleet.forward`); `agent` when it
    /// speaks for an agent there.
    Delegated { agent: bool },
}

/// What is asked of a schedule that exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Act {
    Update,
    Pause,
    Resume,
    Delete,
    RunNow,
}

fn forbidden(why: &str) -> AutomationError {
    AutomationError::Forbidden(why.to_owned())
}

impl Asker {
    /// May this asker make a schedule, given how many there are: `own`
    /// its own (an agent's session's), `people` and `agents` on the node.
    pub(crate) fn may_create(
        &self,
        own: usize,
        people: usize,
        agents: usize,
    ) -> Result<(), AutomationError> {
        match self.via {
            Via::Delegated { .. } => Err(forbidden(
                "a schedule is made on its node by that node's own clients: a call passed on by \
                 another node cannot be acted out again when the schedule runs",
            )),
            Via::Agent if own >= AGENT_SCHEDULES => Err(forbidden(
                "an agent's session has at most 8 schedules (ADR 0033)",
            )),
            Via::Agent if agents >= AGENT_SCHEDULES_ON_NODE => Err(AutomationError::Conflict(
                "the node has as many agents' schedules as it keeps (64)".to_owned(),
            )),
            Via::Person if people >= PEOPLE_SCHEDULES => Err(AutomationError::Conflict(
                "the node has as many schedules as it keeps (256); delete some first".to_owned(),
            )),
            Via::Agent | Via::Person => Ok(()),
        }
    }

    /// May this asker `act` on a schedule `defined_by` last wrote.
    pub(crate) fn may(&self, act: Act, defined_by: &Creator) -> Result<(), AutomationError> {
        match (self.via, act) {
            (Via::Person, _)
            | (Via::Delegated { agent: false }, Act::Pause | Act::Resume | Act::Delete) => Ok(()),
            (Via::Delegated { .. }, _) => Err(forbidden(
                "a call passed on by another node may pause, resume and delete a schedule, not \
                 change or run it",
            )),
            (Via::Agent, Act::RunNow) => Err(forbidden(
                "an agent cannot run a schedule now: it would send its own heartbeat past its \
                 cadence",
            )),
            (Via::Agent, Act::Update | Act::Pause | Act::Resume | Act::Delete) => {
                if *defined_by == self.by {
                    Ok(())
                } else {
                    Err(forbidden("an agent manages only the schedules it made"))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use vessel_proto::{DeviceId, NodeId, SessionId};

    use super::*;

    fn agent(session: SessionId) -> Asker {
        Asker {
            by: Creator::Agent {
                node: NodeId::random(),
                session,
            },
            via: Via::Agent,
        }
    }

    #[test]
    fn agents_are_held_to_their_own_and_to_the_bounds() {
        let session = SessionId::random();
        let asker = agent(session);
        assert!(asker.may_create(7, 300, 10).is_ok());
        assert!(matches!(
            asker.may_create(8, 0, 0),
            Err(AutomationError::Forbidden(_))
        ));
        assert!(matches!(
            asker.may_create(0, 0, 64),
            Err(AutomationError::Conflict(_))
        ));
        assert!(asker.may(Act::Pause, &asker.by).is_ok());
        assert!(asker.may(Act::Update, &Creator::Owner).is_err());
        assert!(asker.may(Act::RunNow, &asker.by).is_err());
    }

    #[test]
    fn people_manage_everything_within_the_bound() {
        let device = Asker {
            by: Creator::Device {
                id: DeviceId::random(),
            },
            via: Via::Person,
        };
        assert!(device.may_create(0, 255, 64).is_ok());
        assert!(device.may_create(0, 256, 0).is_err());
        for act in [
            Act::Update,
            Act::Pause,
            Act::Resume,
            Act::Delete,
            Act::RunNow,
        ] {
            assert!(device.may(act, &Creator::Owner).is_ok(), "{act:?}");
        }
    }

    #[test]
    fn calls_passed_on_may_only_stop_things() {
        let passed = Asker {
            by: Creator::Owner,
            via: Via::Delegated { agent: false },
        };
        assert!(passed.may_create(0, 0, 0).is_err());
        assert!(passed.may(Act::Pause, &Creator::Owner).is_ok());
        assert!(passed.may(Act::Delete, &Creator::Owner).is_ok());
        assert!(passed.may(Act::Update, &Creator::Owner).is_err());
        assert!(passed.may(Act::RunNow, &Creator::Owner).is_err());
        let agent_there = Asker {
            via: Via::Delegated { agent: true },
            ..passed
        };
        assert!(agent_there.may(Act::Pause, &Creator::Owner).is_err());
    }
}
