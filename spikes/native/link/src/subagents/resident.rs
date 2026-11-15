//! Staged resident envelope: no live networking without W12's actual read minter.
//! Canonical binding/ticket admission precedes any owner resource lookup.
use super::{Answer, Ask, Reply};
use vessel_app_view::{
    devices::Ticket,
    residents::{FollowBinding, OperationKind, OperationTicket, ResidentProblem, Residents},
};
use vessel_proto::subagents::ProviderChildRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProjectionCapture {
    pub(super) source: ProviderChildRef,
    pub(super) installed_revision: Option<Ticket>,
}
#[derive(Debug, Clone)]
pub(super) struct ResidentAsk {
    pub(super) binding: FollowBinding,
    pub(super) operation: OperationTicket,
    pub(super) projection: Option<ProjectionCapture>,
    pub(super) request: Ask,
}
#[derive(Debug)]
pub(super) struct ResidentReply {
    binding: FollowBinding,
    operation: OperationTicket,
    projection: Option<ProjectionCapture>,
    answer: Answer,
}
impl ResidentAsk {
    fn answered(self, answer: Answer) -> ResidentReply {
        ResidentReply {
            binding: self.binding,
            operation: self.operation,
            projection: self.projection,
            answer,
        }
    }
}
impl ResidentReply {
    /// Validation converts stale delivery to a safe correlated refusal; canonical
    /// owner admission then settles once before the caller can look up a bundle.
    fn take(
        self,
        residents: &mut Residents,
    ) -> Result<(Reply, Option<ProjectionCapture>), ResidentProblem> {
        let reply = self.answer.into_reply();
        residents.settle_operation(self.binding, OperationKind::Subagents, self.operation)?;
        Ok((reply, self.projection))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use vessel_app_view::{
        connection::ConnectionEpochs,
        devices::{MachineKey, Tickets},
        layout::{
            AgentRole, HostAlias, Layout, LayoutKey, LayoutMode, OpenIntent, PaneId, Placement,
            Target as LayoutTarget,
        },
        residents::{OwnerReadiness, PanelClaim, ResidentToken, ResourceOwner, SessionAddress},
        subagents::{Target, TrackProblem},
    };
    use vessel_proto::{NodeId, SessionId};
    fn fixture() -> (Residents, ResidentToken, PanelClaim, Target) {
        let mut residents = Residents::new();
        let node = NodeId::random();
        let session = SessionId::random();
        let mut layout = Layout::default();
        let tab = layout
            .open(
                LayoutTarget::Agent(session),
                OpenIntent::Reveal,
                Placement::Focused,
                AgentRole::Unknown,
                LayoutMode::SplitPanes,
            )
            .unwrap();
        let claim = PanelClaim {
            workspace: LayoutKey::Session {
                machine: HostAlias::parse("fixture").unwrap(),
                session,
            },
            tab,
        };
        let token = residents
            .claim(
                SessionAddress { node, session },
                claim.clone(),
                PaneId::MAIN,
            )
            .unwrap();
        let context = ConnectionEpochs::default()
            .admit(MachineKey::from_book("fixture"), node)
            .unwrap();
        (residents, token, claim, Target::new(context, session))
    }
    fn pending(residents: &mut Residents, binding: FollowBinding, target: Target) -> ResidentAsk {
        let operation = residents
            .admit_operation(binding, OperationKind::Subagents)
            .unwrap();
        let request = Ask::Lookup {
            target,
            ticket: Tickets::default().next_ticket().unwrap(),
        };
        ResidentAsk {
            binding,
            operation,
            projection: None,
            request,
        }
    }
    fn refusal(request: ResidentAsk) -> ResidentReply {
        let answer = Answer::refused(request.request.clone(), TrackProblem::Unsupported);
        request.answered(answer)
    }
    #[test]
    fn a_retired_attempt_cannot_settle_a_new_attempt_even_when_query_tickets_restart() {
        let (mut residents, token, _claim, target) = fixture();
        let old = residents.begin_follow(token).unwrap();
        let stale = pending(&mut residents, old, target.clone());
        residents.retire_follow(old).unwrap();
        let current = residents.begin_follow(token).unwrap();
        let fresh = pending(&mut residents, current, target);
        assert!(matches!(
            refusal(stale).take(&mut residents),
            Err(ResidentProblem::StaleBinding)
        ));
        assert!(refusal(fresh).take(&mut residents).is_ok());
    }
    #[test]
    fn a_recreated_parent_incarnation_cannot_receive_an_old_child_reply() {
        let (mut residents, token, claim, target) = fixture();
        let old = residents.begin_follow(token).unwrap();
        let stale = pending(&mut residents, old, target.clone());
        residents.retire_follow(old).unwrap();
        for owner in [
            ResourceOwner::Timeline,
            ResourceOwner::Subagents,
            ResourceOwner::Composer,
            ResourceOwner::Attachments,
            ResourceOwner::Requests,
            ResourceOwner::Lifecycle,
        ] {
            residents
                .readiness(token, owner, OwnerReadiness::Ready)
                .unwrap();
        }
        residents.retire_clean(token).unwrap();
        let next = residents.claim(token.address, claim, PaneId::MAIN).unwrap();
        assert_ne!(next.epoch, token.epoch);
        let binding = residents.begin_follow(next).unwrap();
        let current = pending(&mut residents, binding, target);
        assert!(matches!(
            refusal(stale).take(&mut residents),
            Err(ResidentProblem::StaleEpoch)
        ));
        assert!(refusal(current).take(&mut residents).is_ok());
    }
    #[test]
    fn another_owner_ticket_and_duplicate_reply_do_not_settle_child_state() {
        let (mut residents, token, _claim, target) = fixture();
        let binding = residents.begin_follow(token).unwrap();
        let mut wrong = pending(&mut residents, binding, target.clone());
        wrong.operation = residents
            .admit_operation(binding, OperationKind::Timeline)
            .unwrap();
        assert!(matches!(
            refusal(wrong).take(&mut residents),
            Err(ResidentProblem::UnadmittedOperation)
        ));
        let current = pending(&mut residents, binding, target);
        let duplicate = current.clone();
        assert!(refusal(current).take(&mut residents).is_ok());
        assert!(matches!(
            refusal(duplicate).take(&mut residents),
            Err(ResidentProblem::UnadmittedOperation)
        ));
    }
    #[test]
    fn projection_capture_remains_the_exact_installed_child_revision() {
        let (mut residents, token, _claim, target) = fixture();
        let binding = residents.begin_follow(token).unwrap();
        let mut request = pending(&mut residents, binding, target.clone());
        let capture = ProjectionCapture {
            source: ProviderChildRef {
                parent: target.address,
                child: "actual-provider-child".parse().unwrap(),
            },
            installed_revision: Some(Tickets::default().next_ticket().unwrap()),
        };
        request.projection = Some(capture.clone());
        let (_reply, returned) = refusal(request).take(&mut residents).unwrap();
        assert_eq!(returned, Some(capture));
    }
}
