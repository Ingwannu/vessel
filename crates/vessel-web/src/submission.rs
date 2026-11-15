//! Prepared browser submissions use the actual SDK support check before uploads.
//! Recipient identity is a downgrade-only constraint, never caller authority.
use crate::registry::Registry;
use crate::source::NodeSource;
use crate::wire::{Failure, NodeKey, SubmitCheck};
use std::sync::Arc;
use vessel_client::CallError;
use vessel_proto::timeline::SubmitPrompt;
use vessel_proto::{NodeId, Request};

pub(crate) fn recipient(actual: NodeId, expected: Option<NodeId>) -> Result<(), Failure> {
    if expected.is_some_and(|expected| expected != actual) {
        return Err(Failure::Refused {
            message: "The selected machine changed before this prepared call.".to_owned(),
        });
    }
    Ok(())
}
pub(crate) async fn check<S: NodeSource>(
    registry: &Arc<Registry<S>>,
    node: &NodeKey,
    expected: NodeId,
    request: SubmitPrompt,
) -> SubmitCheck {
    let client = match registry.client(node).await {
        Ok(client) => client,
        Err(failure) => return SubmitCheck::Refused { failure },
    };
    if let Err(failure) = recipient(client.node().id, Some(expected)) {
        return SubmitCheck::Refused { failure };
    }
    // check_call reserves no RPC id and sends no frame. Input/ownership/provider
    // capability validation still happens on the one actual SubmitPrompt call.
    match client.check_call(&request.into_call()) {
        Ok(()) => SubmitCheck::Ready {
            node: client.node().id,
        },
        Err(error @ (CallError::NotInProtocol { .. } | CallError::UnassignedProtocol { .. })) => {
            SubmitCheck::Unsupported {
                failure: crate::calls::failure(error),
            }
        }
        Err(
            error @ (CallError::Remote(_)
            | CallError::Disconnected
            | CallError::UnexpectedReply(_)
            | CallError::RequestIdsExhausted),
        ) => SubmitCheck::Refused {
            failure: crate::calls::failure(error),
        },
    }
}
