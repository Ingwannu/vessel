//! Side-question policy and bounded context, never an alternate agent
//! transport. A side is an ordinary isolated native conversation; only its
//! dedicated prompt ingress may add refreshed parent context (ADR 0045).

mod snapshot;

pub(crate) use snapshot::prompt;

use vessel_proto::{ErrorCode, Method, WireError};

use crate::principal::Principal;

/// Refusals whose meaning survives the service's wire mapping.
#[derive(Debug, thiserror::Error)]
pub(crate) enum SideError {
    #[error("side questions are for a person; agents can start ordinary sessions instead")]
    Agent,
    #[error("side questions need read and operate access; ask the owner to grant both")]
    ReadScope,
    #[error("open a side question from a conversation, not from a terminal")]
    Terminal,
    #[error("a side conversation cannot have its own side question; open the original session")]
    Nested,
    #[error("this is not a side conversation; open one with session.side_open first")]
    NotSide,
    #[error(
        "send side questions with session.side_prompt or vessel session side; ordinary prompts and queues cannot refresh their parent context"
    )]
    PromptOnly,
    #[error("this side changed while the action was pending; open the side conversation again")]
    Changed,
    #[error("a side conversation cannot be forked; start over with session.side_reset instead")]
    Fork,
    #[error("side questions cannot be used as scheduled or saved session templates")]
    Template,
    #[error("the side question and its context exceed the prompt limit; ask a shorter question")]
    TooLarge,
    #[error(
        "native slash commands cannot carry side context; run the command in an ordinary conversation"
    )]
    Slash,
    #[error("the side context could not be prepared")]
    Context(#[source] Box<vessel_agent::InputError>),
}

impl SideError {
    pub(crate) const fn code(&self) -> ErrorCode {
        match self {
            Self::Agent | Self::ReadScope => ErrorCode::Forbidden,
            Self::Terminal | Self::Nested | Self::PromptOnly | Self::Slash | Self::Fork => {
                ErrorCode::Unsupported
            }
            Self::Changed => ErrorCode::Conflict,
            Self::NotSide | Self::Template | Self::TooLarge | Self::Context(_) => {
                ErrorCode::InvalidRequest
            }
        }
    }

    pub(crate) fn wire(&self) -> WireError {
        WireError::new(self.code(), self.to_string())
    }
}

/// Also used by ordinary creation naming a side link and explicit resume,
/// so neither a forwarded agent nor an operate-only device bypasses it.
pub(crate) fn person(principal: &Principal) -> Result<(), SideError> {
    if principal.agent().is_some() {
        return Err(SideError::Agent);
    }
    if principal.permit(Method::GetConversation).is_err() {
        return Err(SideError::ReadScope);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_proto::{DeviceId, NodeId, Origin, OriginBy, Scope, SessionId};

    #[test]
    fn delegated_people_keep_the_intersection_and_delegated_agents_are_refused() {
        let origin = Origin {
            node: NodeId::random(),
            by: OriginBy::Owner,
            scopes: vec![Scope::Read, Scope::Operate],
        };
        let person_call = Principal::delegated(
            DeviceId::random(),
            &[Scope::Read, Scope::Operate],
            origin.clone(),
            NodeId::random(),
        );
        assert!(person(&person_call).is_ok());
        let insufficient = Principal::delegated(
            DeviceId::random(),
            &[Scope::Operate],
            origin.clone(),
            NodeId::random(),
        );
        assert_eq!(
            person(&insufficient).unwrap_err().code(),
            ErrorCode::Forbidden
        );
        let agent_call = Principal::delegated(
            DeviceId::random(),
            &[Scope::Read, Scope::Operate],
            Origin {
                by: OriginBy::Agent {
                    session: SessionId::random(),
                    depth: 1,
                },
                ..origin
            },
            NodeId::random(),
        );
        assert_eq!(
            person(&agent_call).unwrap_err().to_string(),
            "side questions are for a person; agents can start ordinary sessions instead"
        );
    }
}
