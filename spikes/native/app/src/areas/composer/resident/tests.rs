use super::*;
use std::sync::Arc;
use vessel_app_link::attachments::WithFiles;
use vessel_app_view::{
    composer::{
        context::{ContextAttachment, ContextId, ContextKind, ContextText, ContextTitle, Payload},
        tray::Origin,
    },
    connection::{ConnectionEpochs, MachineKey},
    layout::{
        AgentRole, HostAlias, Layout, LayoutKey, LayoutMode, OpenIntent, PaneId, Placement, Target,
    },
    machine_book::Name,
    residents::{PanelClaim, Residents, SessionAddress},
};
use vessel_proto::{Delivery, NodeId, SessionId};

fn fixture() -> (Residents, SessionContext) {
    let address = SessionAddress {
        node: NodeId::random(),
        session: SessionId::random(),
    };
    let mut layout = Layout::default();
    let tab = layout
        .open(
            Target::Agent(address.session),
            OpenIntent::New,
            Placement::Focused,
            AgentRole::Unknown,
            LayoutMode::SplitPanes,
        )
        .unwrap();
    let claim = PanelClaim {
        workspace: LayoutKey::Session {
            machine: HostAlias::parse("fixture").unwrap(),
            session: address.session,
        },
        tab,
    };
    let mut residents = Residents::new();
    let token = residents
        .claim(address, claim.clone(), PaneId::MAIN)
        .unwrap();
    let context = residents.context(token, &claim).unwrap();
    (residents, context)
}
fn waiting(context: &SessionContext) -> (Box<Waiting>, Arc<WithFiles>) {
    let address = context.resident.address;
    let connection = ConnectionEpochs::default()
        .admit(MachineKey::from_book("fixture"), address.node)
        .unwrap();
    let message = vessel_proto::timeline::ClientMessageId::random();
    let machine = Name::parse("fixture").unwrap();
    let payload = Arc::new(WithFiles {
        connection: connection.clone(),
        message: message.clone(),
        machine: machine.clone(),
        expected_node: address.node,
        session: address.session,
        text: "protected input".into(),
        context: None,
        attachments: Vec::new(),
        delivery: Delivery::Start,
    });
    (
        Box::new(Waiting {
            connection,
            message,
            machine,
            expected_node: address.node,
            text: payload.text.clone(),
            context: None,
            phase: Phase::Submitting(Arc::clone(&payload)),
            local_files: Vec::new(),
            delivery: Delivery::Start,
            creation: None,
        }),
        payload,
    )
}

#[test]
fn clean_snapshot_moves_only_within_its_registry_and_authenticated_address() {
    let (_, context) = fixture();
    let resources = SessionAttachments::new(&context);
    assert_eq!(resources.readiness(), OwnerReadiness::Ready);
    let SnapshotAttempt::Ready(snapshot) = resources.into_snapshot() else {
        panic!("clean snapshot");
    };
    let (_, foreign) = fixture();
    let RestoreAttempt::Refused(snapshot) = snapshot.restore(&foreign) else {
        panic!("foreign refusal");
    };
    let RestoreAttempt::Ready(restored) = snapshot.restore(&context) else {
        panic!("same owner");
    };
    assert_eq!(restored.owner, context.resident);
    assert_eq!(restored.readiness(), OwnerReadiness::Ready);
}
#[test]
fn suppressed_context_and_byte_leases_refuse_snapshot_without_losing_the_resource() {
    let (_, context) = fixture();
    let mut resources = SessionAttachments::new(&context);
    let item = ContextAttachment::Snapshot {
        kind: ContextKind::Review,
        payload: Payload {
            id: ContextId::parse("review").unwrap(),
            title: ContextTitle::parse("Review").unwrap(),
            text: ContextText::parse("retained source").unwrap(),
            url: None,
        },
    };
    resources.context.add(item.clone()).unwrap();
    resources.context.remove(&item);
    let bytes: Arc<[u8]> = Arc::from([1, 2, 3]);
    let key = resources
        .tray
        .add("image.png", 3, Origin::Pasted(Arc::clone(&bytes)))
        .unwrap();
    let SnapshotAttempt::Held {
        mut resources,
        reason,
    } = resources.into_snapshot()
    else {
        panic!("dirty resources must stay");
    };
    assert_eq!(reason, HoldReason::DirtyInput);
    assert!(resources.context.has_retained());
    assert_eq!(resources.context.visible().count(), 0);
    let [item] = resources.tray.items() else {
        panic!("one retained byte lease");
    };
    assert!(matches!(&item.origin, Origin::Pasted(current) if Arc::ptr_eq(current, &bytes)));
    assert!(resources.tray.remove(key));
    assert_eq!(
        resources.readiness(),
        OwnerReadiness::Hold(HoldReason::DirtyInput)
    );
}
#[test]
fn local_loading_overlay_and_unconfirmed_recovery_remain_distinct_holds() {
    let (_, context) = fixture();
    let mut resources = SessionAttachments::new(&context);
    resources.acquiring = NonZeroUsize::new(1);
    assert_eq!(
        resources.readiness(),
        OwnerReadiness::Hold(HoldReason::Loading)
    );
    resources.acquiring = None;
    resources.preview = Preview::Held;
    assert_eq!(
        resources.readiness(),
        OwnerReadiness::Hold(HoldReason::Overlay)
    );
    resources.recovery = Recovery::Unconfirmed;
    assert_eq!(
        resources.readiness(),
        OwnerReadiness::Hold(HoldReason::UncertainEffect)
    );
    let (waiting, _) = waiting(&context);
    resources.recovery = Recovery::Confirmed;
    resources.submission = Submission::Prepared(waiting);
    assert_eq!(
        resources.readiness(),
        OwnerReadiness::Hold(HoldReason::PendingEffect)
    );
}
#[test]
fn retirement_matches_the_original_ticket_and_retains_exact_payload_without_replay() {
    let (mut residents, context) = fixture();
    let first = residents.begin_follow(context.resident).unwrap();
    let ticket = residents
        .admit_operation(first, OperationKind::Attachment)
        .unwrap();
    let notice = residents.retire_current(context.resident).unwrap().unwrap();
    let second = residents.begin_follow(context.resident).unwrap();
    let current = residents
        .admit_operation(second, OperationKind::Attachment)
        .unwrap();
    let mut resources = SessionAttachments::new(&context);
    let (pending, payload) = waiting(&context);
    let message = payload.message.clone();
    resources.submission = Submission::Active {
        binding: second,
        ticket: current,
        waiting: pending,
    };
    assert!(!resources.follow_retired(&notice));
    assert_eq!(resources.submission.operation(), Some((second, current)));
    let current_notice = residents.retire_current(context.resident).unwrap().unwrap();
    assert!(resources.follow_retired(&current_notice));
    assert!(!resources.follow_retired(&current_notice));
    assert_eq!(
        resources.readiness(),
        OwnerReadiness::Hold(HoldReason::UncertainEffect)
    );
    let SnapshotAttempt::Held { resources, reason } = resources.into_snapshot() else {
        panic!("unknown outcome stays protected");
    };
    assert_eq!(reason, HoldReason::UncertainEffect);
    let pending = resources.submission.waiting().unwrap();
    assert_eq!(pending.message, message);
    assert!(
        matches!(&pending.phase, Phase::Submitting(retained) if Arc::ptr_eq(retained, &payload))
    );
    assert_ne!(ticket, current);
}
