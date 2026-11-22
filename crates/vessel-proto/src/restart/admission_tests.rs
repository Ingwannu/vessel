//! The optional fields share canonical recursive admission without a local version.
use crate::settings::{GetNodeSettings, SetNodeSettings};
use crate::{Call, Deliver, Forward, NodeId, RelayCall, Request as _};
fn envelopes(call: Call) -> [Call; 4] {
    [
        call.clone(),
        Forward {
            node: NodeId::random(),
            call: Box::new(call.clone()),
        }
        .into_call(),
        RelayCall {
            node: NodeId::random(),
            call: Box::new(call.clone()),
            origin: crate::rpc::samples::origin(),
        }
        .into_call(),
        Deliver {
            call: Box::new(call),
            origin: crate::rpc::samples::origin(),
        }
        .into_call(),
    ]
}
#[test]
fn explicit_recovery_fields_use_the_same_boundary_in_every_envelope() {
    let get = GetNodeSettings {
        include_restart_recovery: true,
    }
    .into_call();
    let set = SetNodeSettings {
        restart_recovery: Some(super::SetRestartRecovery::default()),
        ..SetNodeSettings::default()
    }
    .into_call();
    for call in [get, set].into_iter().flat_map(envelopes) {
        assert_eq!(call.since(), super::RESTART_RECOVERY_PROTOCOL);
        assert_eq!(call.required_protocol(), super::RESTART_RECOVERY_PROTOCOL);
    }
    for call in envelopes(GetNodeSettings::default().into_call()) {
        assert!(call.required_protocol().is_some());
    }
}
