//! DTO construction/decoding does not reach a private slot or service runtime.

use super::*;
use crate::WorkspaceId;

fn exposure(workspace: WorkspaceId, ordinal: usize) -> ServiceExposureView {
    ServiceExposureView {
        target: WorkspaceScriptRef {
            workspace,
            script: format!("web-{ordinal}").parse().unwrap(),
        },
        choice: ServiceExposureChoice::Private,
        effective: ServiceExposureEffective::Private,
    }
}

#[test]
fn admin_choice_count_and_unique_targets_hold_at_constructor_and_decode() {
    let workspace = WorkspaceId::random();
    let entries: Vec<_> = (0..MAX_PUBLIC_SERVICE_CHOICES)
        .map(|index| exposure(workspace, index))
        .collect();
    let settings =
        ServicesAdminSettings::new(None, ServiceGateway::LocalOnly {}, entries.clone()).unwrap();
    let text = serde_json::to_string(&settings).unwrap();
    assert_eq!(
        serde_json::from_str::<ServicesAdminSettings>(&text).unwrap(),
        settings
    );
    let mut too_many = entries;
    too_many.push(exposure(workspace, MAX_PUBLIC_SERVICE_CHOICES));
    let oversized = format!(
        r#"{{"gateway":{{"kind":"local_only"}},"exposures":{}}}"#,
        serde_json::to_string(&too_many).unwrap()
    );
    assert!(serde_json::from_str::<ServicesAdminSettings>(&oversized).is_err());
    let error =
        ServicesAdminSettings::new(None, ServiceGateway::LocalOnly {}, too_many).unwrap_err();
    assert_eq!(error, InvalidServicesAdminSettings::TooMany);
    assert!(
        error
            .to_string()
            .contains(&MAX_PUBLIC_SERVICE_CHOICES.to_string())
    );
    let entry = exposure(workspace, 0);
    assert_eq!(
        ServicesAdminSettings::new(
            None,
            ServiceGateway::LocalOnly {},
            vec![entry.clone(), entry.clone()]
        ),
        Err(InvalidServicesAdminSettings::Duplicate)
    );
    let json = format!(
        r#"{{"gateway":{{"kind":"local_only"}},"exposures":[{0},{0}]}}"#,
        serde_json::to_string(&entry).unwrap()
    );
    assert!(serde_json::from_str::<ServicesAdminSettings>(&json).is_err());
    let ServicesAdminSettingsView {
        revision,
        gateway,
        exposures,
    } = settings.view();
    assert_eq!(revision, None);
    assert_eq!(gateway, &ServiceGateway::LocalOnly {});
    assert_eq!(exposures.len(), MAX_PUBLIC_SERVICE_CHOICES);
}

#[test]
fn anonymous_enable_requires_explicit_warning_and_contains_no_binding_authority() {
    let change = ServiceExposureChange::AnonymousPublic {
        snapshot: ServiceSnapshotId::random(),
        definition: CommandsDigest::from_sha256(Default::default()),
        acknowledgement: PublicServiceAcknowledgement::ReachableWithoutVesselAuthentication,
    };
    let request = SetServiceExposure {
        target: exposure(WorkspaceId::random(), 0).target,
        expected: None,
        change,
    };
    let json = serde_json::to_string(&request).unwrap();
    assert_eq!(
        serde_json::from_str::<SetServiceExposure>(&json).unwrap(),
        request
    );
    assert!(json.contains("reachable_without_vessel_authentication"));
    for field in ["incarnation", "pid", "url", "run", "token"] {
        assert!(!json.contains(&format!("\"{field}\"")));
    }
    let ServiceExposureChange::AnonymousPublic {
        snapshot,
        definition,
        acknowledgement: _,
    } = request.change
    else {
        panic!("fixture changed")
    };
    let missing = format!(
        r#"{{"kind":"anonymous_public","snapshot":"{snapshot}","definition":"{definition}"}}"#
    );
    assert!(serde_json::from_str::<ServiceExposureChange>(&missing).is_err());
    let implicit = format!(
        r#"{{"kind":"anonymous_public","snapshot":"{snapshot}","definition":"{definition}","acknowledgement":true}}"#
    );
    assert!(serde_json::from_str::<ServiceExposureChange>(&implicit).is_err());
    for json in [
        r#"{"kind":"future"}"#,
        r#"{"kind":"private","incarnation":"synthetic"}"#,
    ] {
        assert!(serde_json::from_str::<ServiceExposureChange>(json).is_err());
    }
}

#[test]
fn private_and_public_changes_refuse_unknown_binding_fields_in_requests() {
    let private = ServiceExposureChange::Private {};
    let public = ServiceExposureChange::AnonymousPublic {
        snapshot: ServiceSnapshotId::random(),
        definition: CommandsDigest::from_sha256(Default::default()),
        acknowledgement: PublicServiceAcknowledgement::ReachableWithoutVesselAuthentication,
    };
    let target = exposure(WorkspaceId::random(), 0).target;
    let target = serde_json::to_string(&target).unwrap();
    assert_eq!(
        serde_json::to_string(&private).unwrap(),
        r#"{"kind":"private"}"#
    );
    for choice in [private, public] {
        let json = serde_json::to_string(&choice).unwrap();
        assert_eq!(
            serde_json::from_str::<ServiceExposureChange>(&json).unwrap(),
            choice
        );
        let fields = json.strip_suffix('}').unwrap();
        for field in ["incarnation", "pid", "url", "run", "token"] {
            let extra = format!(r#"{fields},"{field}":"synthetic"}}"#);
            assert!(serde_json::from_str::<ServiceExposureChange>(&extra).is_err());
            let request = format!(r#"{{"target":{target},"expected":null,"change":{extra}}}"#);
            assert!(serde_json::from_str::<SetServiceExposure>(&request).is_err());
        }
    }
}

#[test]
fn requested_effective_and_application_states_are_not_collapsed() {
    let revision = ServicesSettingsRevision::random();
    let settings =
        ServicesAdminSettings::new(Some(revision), ServiceGateway::LocalOnly {}, vec![]).unwrap();
    let request = SetServiceGateway {
        expected: Some(revision),
        gateway: ServiceGateway::LocalOnly {},
    };
    let text = serde_json::to_string(&request).unwrap();
    assert_eq!(
        serde_json::from_str::<SetServiceGateway>(&text).unwrap(),
        request
    );
    assert_eq!(settings.view().revision, Some(revision));
    let mut observed = exposure(WorkspaceId::random(), 0);
    observed.choice = ServiceExposureChoice::AnonymousPublic;
    observed.effective = ServiceExposureEffective::Inactive {
        reason: ServiceUnavailable::IncarnationUnavailable,
    };
    let text = serde_json::to_string(&observed).unwrap();
    assert_eq!(
        serde_json::from_str::<ServiceExposureView>(&text).unwrap(),
        observed
    );
    assert_eq!(
        serde_json::from_str::<ServiceExposureEffective>(r#"{"kind":"future"}"#).unwrap(),
        ServiceExposureEffective::Unknown
    );
}
