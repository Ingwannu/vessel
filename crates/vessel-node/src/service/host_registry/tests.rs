use super::*;
use vessel_proto::{
    NodeId, ProtocolVersion,
    host_registry::{HostMetadata, HostRegistrationId, HostRevision},
};

#[test]
fn fleet_reachability_never_claims_a_paired_route_with_the_same_node_id_is_online() {
    let node = NodeId::random();
    let fleet = Ok(vec![FleetNode {
        node,
        name: "fleet observation".to_owned(),
        protocol: ProtocolVersion::new(25),
        reachable: true,
    }]);
    let mut record = HostRegistration {
        registration_id: HostRegistrationId::random(),
        source: HostSource::Owner,
        destination: HostDestination::Paired { node },
        label: None,
        metadata: HostMetadata::default(),
        enabled: HostEnabled::Enabled,
        revision: HostRevision::default(),
    };
    let paired = summary(record.clone(), &fleet);
    assert_eq!(paired.status, HostStatus::Error);
    assert_eq!(paired.error.unwrap().code, ErrorCode::NotFound);
    assert!(paired.destination_name.is_none());

    record.destination = HostDestination::Fleet { node };
    let actual_fleet = summary(record.clone(), &fleet);
    assert_eq!(actual_fleet.status, HostStatus::Online);
    assert_eq!(
        actual_fleet.destination_name.unwrap().as_str(),
        "fleet observation"
    );
    assert!(actual_fleet.error.is_none());

    record.destination = HostDestination::Paired { node };
    record.enabled = HostEnabled::Disabled;
    let disabled = summary(record, &fleet);
    assert_eq!(disabled.status, HostStatus::Disabled);
    assert!(disabled.error.is_none());
}
