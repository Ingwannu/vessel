use super::*;
use crate::rpc::{Method, Request};

// Decode the actual output into an independent closed schema. Extra fields must fail.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Allowed {
    registration_id: HostRegistrationId,
    destination: HostDestination,
    destination_name: Option<HostLabel>,
    label: Option<HostLabel>,
    annotations: HostMetadata,
}

fn row() -> SharedHostRegistration {
    SharedHostRegistration {
        registration_id: HostRegistrationId::random(),
        destination: HostDestination::Fleet {
            node: NodeId::random(),
        },
        destination_name: Some("observed".parse().unwrap()),
        label: Some("public label".parse().unwrap()),
        annotations: HostMetadata::default(),
    }
}

#[test]
fn public_inventory_has_only_the_approved_serialized_fields() {
    let row = row();
    let encoded = serde_json::to_string(&row).unwrap();
    let allowed: Allowed = serde_json::from_str(&encoded).unwrap();
    assert_eq!(allowed.registration_id, row.registration_id);
    assert_eq!(allowed.destination, row.destination);
    assert_eq!(allowed.destination_name, row.destination_name);
    assert_eq!(allowed.label, row.label);
    assert_eq!(allowed.annotations, row.annotations);
    for field in [
        "source",
        "revision",
        "credentials",
        "config",
        "enabled",
        "status",
    ] {
        let extra = encoded.replacen('{', &format!("{{\"{field}\":null,"), 1);
        assert!(serde_json::from_str::<SharedHostRegistration>(&extra).is_err());
    }
    let directory = SharedHostDirectory {
        node: NodeId::random(),
        hosts: vec![row].try_into().unwrap(),
    };
    let roundtrip: SharedHostDirectory =
        serde_json::from_str(&serde_json::to_string(&directory).unwrap()).unwrap();
    assert_eq!(roundtrip, directory);
    assert!(
        !serde_json::to_string(&directory)
            .unwrap()
            .contains("hostname")
    );
}

#[test]
fn public_inventory_bounds_apply_to_construction_and_wire_decoding() {
    let rows = vec![row(); MAX_HOST_REGISTRATIONS];
    assert!(SharedHostRegistrations::try_from(rows.clone()).is_ok());
    let mut excess = rows;
    excess.push(row());
    assert_eq!(
        SharedHostRegistrations::try_from(excess.clone()),
        Err(TooManySharedHosts)
    );
    assert!(
        serde_json::from_str::<SharedHostRegistrations>(&serde_json::to_string(&excess).unwrap())
            .is_err()
    );
    let text = format!(
        "{{\"registration_id\":\"{}\",\"destination\":{{\"kind\":\"fleet\",\"node\":\"{}\"}},\"annotations\":{{\"note\":\"{}\"}}}}",
        HostRegistrationId::random(),
        NodeId::random(),
        "x".repeat(super::super::METADATA_VALUE_CHARS.checked_add(1).unwrap())
    );
    assert!(serde_json::from_str::<SharedHostRegistration>(&text).is_err());
}

#[test]
fn public_inventory_is_read_only_pending_and_nonforwardable() {
    let call = ListSharedHostRegistrations.into_call();
    assert_eq!(call.method(), Method::ListSharedHostRegistrations);
    assert_eq!(call.since(), super::super::CLI_PARITY_PROTOCOL);
    assert_eq!(call.method().scope(), Some(crate::Scope::Read));
    assert!(!call.method().forwardable());
}

#[test]
fn observed_names_preserve_destination_authority_kind_instead_of_only_node_identity() {
    let fleet = row();
    let mut paired = fleet.clone();
    paired.registration_id = HostRegistrationId::random();
    paired.destination = HostDestination::Paired {
        node: fleet.destination.node(),
    };
    let mut hosts: SharedHostRegistrations = vec![fleet.clone(), paired].try_into().unwrap();
    hosts.observe_names(|destination| {
        (destination == fleet.destination).then(|| "fleet name".parse().unwrap())
    });
    assert_eq!(
        hosts.entries()[0]
            .destination_name
            .as_ref()
            .unwrap()
            .as_str(),
        "fleet name"
    );
    assert!(hosts.entries()[1].destination_name.is_none());
}
