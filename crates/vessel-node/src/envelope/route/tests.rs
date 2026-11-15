//! Exact authorized wire frames, without claiming a real assigned callback fleet.
//! Links still owns direct/callback connection selection; these fixtures pin what
//! both paths carry so cleanup cannot silently become an owner call.

use super::*;
use crate::principal::Principal;
use crate::transfers::Holder;
use vessel_proto::{DeviceId, ExportSession, OriginBy, Scope, SessionId};

fn unpack(frame: Frame) -> (NodeId, Origin, Box<Call>) {
    match frame {
        Frame::Member { node, request } => (node, request.origin, request.call),
        Frame::Home(request) => (request.node, request.origin, request.call),
    }
}

#[test]
fn forward_relay_and_callback_member_frames_keep_the_exact_origin_and_transfer() {
    let target = NodeId::random();
    let source = NodeId::random();
    let device = DeviceId::random();
    let transfer = TransferId::random();
    for by in [
        OriginBy::Owner,
        OriginBy::Device {
            id: DeviceId::random(),
        },
        OriginBy::Agent {
            session: SessionId::random(),
            depth: 1,
        },
    ] {
        let origin = Origin {
            node: source,
            by,
            scopes: vec![Scope::Read, Scope::Operate],
        };
        // Forward via home, Relay via member, and the same member delivery when
        // Links selects callback all carry the same exact principal and handle.
        for hop in [Hop::Home, Hop::Member] {
            let export = Box::new(
                ExportSession {
                    session: SessionId::random(),
                    force: false,
                }
                .into_call(),
            );
            let (opened_at, opener, _) = unpack(frame(hop, target, &origin, export));
            let close = Box::new(CloseTransfer { transfer }.into_call());
            let (closed_at, closer, call) = unpack(frame(hop, target, &origin, close));
            assert_eq!(opened_at, target);
            assert_eq!(closed_at, opened_at);
            assert_eq!(
                opener, origin,
                "export must retain the input principal and scopes"
            );
            assert_eq!(closer, opener);
            let original = Principal::delegated(device, &Scope::ALL, opener, NodeId::random());
            let cleanup = Principal::delegated(device, &Scope::ALL, closer, NodeId::random());
            assert_eq!(Holder::of(&original), Holder::of(&cleanup));
            let Call::CloseTransfer(request) = *call else {
                panic!("expected exact close");
            };
            assert_eq!(request.transfer, transfer);
        }
    }
}
