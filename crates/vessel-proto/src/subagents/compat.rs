//! New unary replies have no honest old-peer shape; refuse through every hop.

use crate::{ErrorCode, ProtocolVersion, Reply, WireError};

pub(crate) fn withheld(reply: &Reply, protocol: ProtocolVersion) -> Option<WireError> {
    match reply {
        Reply::DetachSession(_) | Reply::ListProviderChildren(_) | Reply::ReadProviderChild(_)
            if reply.method().since().is_none_or(|since| since > protocol) =>
        {
            Some(WireError::new(
                ErrorCode::Unsupported,
                "this client cannot read the subagent relationship protocol",
            ))
        }
        Reply::Forward(forwarded) | Reply::Relay(forwarded) | Reply::Deliver(forwarded) => {
            withheld(&forwarded.reply, protocol)
        }
        // Other domains retain their existing central projection policy.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Call, Forward, Forwarded, Method, NodeId, Origin, OriginBy, RelayCall, Scope, SessionId,
        subagents::*,
    };

    #[test]
    fn provider_calls_and_nested_replies_never_bypass_pending_admission() {
        let session = SessionId::random();
        let calls = [
            Call::ListProviderChildren(ListProviderChildren { session }),
            Call::ReadProviderChild(ReadProviderChild {
                session,
                child: "opaque-child".parse().unwrap(),
                cursor: None,
                limit: ChildLimit::default(),
            }),
        ];
        for call in calls {
            assert_eq!(call.method().scope(), Some(Scope::Read));
            assert!(call.method().forwardable());
            assert_eq!(call.required_protocol(), SUBAGENT_RELATIONS);
            let forwarded = Call::Forward(Forward {
                node: NodeId::random(),
                call: Box::new(call),
            });
            assert_eq!(forwarded.required_protocol(), SUBAGENT_RELATIONS);
            let relay = Call::Relay(RelayCall {
                node: NodeId::random(),
                origin: Origin {
                    node: NodeId::random(),
                    by: OriginBy::Owner,
                    scopes: vec![Scope::Read],
                },
                call: Box::new(forwarded),
            });
            assert_eq!(relay.required_protocol(), SUBAGENT_RELATIONS);
        }
        assert_eq!(Method::ReadProviderChild.since(), SUBAGENT_RELATIONS);
        let reply = Reply::ListProviderChildren(ProviderChildren {
            children: Vec::new().try_into().unwrap(),
            history: ChildHistory::Complete,
        });
        let wrapped = Reply::Forward(Forwarded {
            reply: Box::new(Reply::Relay(Forwarded {
                reply: Box::new(Reply::Deliver(Forwarded {
                    reply: Box::new(reply.clone()),
                })),
            })),
        });
        for protocol in [crate::MIN_PROTOCOL_VERSION, crate::PROTOCOL_VERSION] {
            assert_eq!(
                reply.withheld(protocol).unwrap().code,
                ErrorCode::Unsupported
            );
            assert_eq!(
                wrapped.withheld(protocol).unwrap().code,
                ErrorCode::Unsupported
            );
        }
    }
}
