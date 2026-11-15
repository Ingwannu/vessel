use super::*;
use vessel_proto::host_registry::{HostDestination, HostRevision};

#[test]
fn concurrent_edit_cannot_replace_the_acknowledged_host_in_the_cli_receipt() {
    let acknowledged = HostRegistration {
        registration_id: HostRegistrationId::random(),
        source: HostSource::Owner,
        destination: HostDestination::Fleet {
            node: NodeId::random(),
        },
        label: Some("my edit".parse().unwrap()),
        metadata: HostMetadata::default(),
        enabled: HostEnabled::Enabled,
        revision: HostRevision::default(),
    };
    let observation = HostSummary {
        registration: acknowledged.clone(),
        status: HostStatus::Offline,
        destination_name: Some("observed name".parse().unwrap()),
        error: None,
    };
    assert_eq!(
        checked_summary(&acknowledged, observation.clone()).unwrap(),
        observation
    );
    let mut newer = observation.clone();
    newer.registration.revision = acknowledged.revision.next().unwrap();
    newer.registration.label = Some("another writer".parse().unwrap());
    let mut rebound = observation.clone();
    rebound.registration.destination = HostDestination::Fleet {
        node: NodeId::random(),
    };
    let mut wrong_id = observation;
    wrong_id.registration.registration_id = HostRegistrationId::random();
    for changed in [newer, rebound] {
        assert!(matches!(
            checked_summary(&acknowledged, changed),
            Err(CliError::ParityHostObservationChanged(id)) if id == acknowledged.registration_id
        ));
    }
    assert!(matches!(
        checked_summary(&acknowledged, wrong_id),
        Err(CliError::ParityInvalidReceipt(_))
    ));
}
