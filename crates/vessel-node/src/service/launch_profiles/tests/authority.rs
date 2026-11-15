//! Scope, origin and durable CAS exercised through the real generated guard.
use super::*;
use crate::{guard::Guarded, principal::Principal, service::Connection};
use vessel_proto::launch_profiles::*;
use vessel_proto::{DeviceId, ErrorCode, Handler as _, HarnessKind, LaunchProfileId, Scope};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn host_profile_scopes_origin_and_cas_are_independent() {
    vessel_testkit::within(
        SAFETY,
        Box::pin(async {
            let fixture = Fixture::new().await;
            let service = fixture.service();
            let owner = fixture.context(Principal::Owner).await;
            let read = fixture
                .context(Principal::Device {
                    id: DeviceId::random(),
                    scopes: vec![Scope::Read],
                })
                .await;
            let no_read = fixture
                .context(Principal::Device {
                    id: DeviceId::random(),
                    scopes: vec![Scope::Admin],
                })
                .await;
            let agent = fixture.context(unrelated_agent()).await;
            let delegated = fixture
                .delegated_agent(fixture.node.as_ref().unwrap().info().id)
                .await;
            let snapshot = check_reads(&service, &read.connection, &no_read.connection).await;
            let request = metadata_request(snapshot.revision());
            for context in [&read.connection, &agent.connection, &delegated.connection] {
                assert_eq!(
                    phase(
                        "scope/origin replacement denied",
                        service.replace_launch_profiles(context, request.clone())
                    )
                    .await
                    .unwrap_err()
                    .code,
                    ErrorCode::Forbidden
                );
            }
            check_committed_metadata(
                &service,
                &owner.connection,
                &no_read.connection,
                &agent.connection,
                request,
            )
            .await;
            check_editor_origins(&service, [&agent.connection, &delegated.connection]).await;
            delegated.finish().await;
            agent.finish().await;
            no_read.finish().await;
            read.finish().await;
            owner.finish().await;
            fixture.finish().await;
        }),
    )
    .await
    .unwrap();
}

async fn check_committed_metadata(
    service: &Guarded,
    owner: &Connection,
    admin: &Connection,
    agent: &Connection,
    request: ReplaceLaunchProfiles,
) {
    let saved = phase(
        "owner saves missing account metadata",
        service.replace_launch_profiles(owner, request.clone()),
    )
    .await
    .unwrap();
    assert_eq!(saved.profiles(), &request.profiles);
    let same = phase(
        "Admin-only device may replace known revision",
        service.replace_launch_profiles(
            admin,
            ReplaceLaunchProfiles {
                expected: saved.revision(),
                profiles: saved.profiles().clone(),
            },
        ),
    )
    .await
    .unwrap();
    assert_eq!(same, saved);

    assert_eq!(
        phase(
            "stale matching content still conflicts",
            service.replace_launch_profiles(owner, request)
        )
        .await
        .unwrap_err()
        .code,
        ErrorCode::Conflict
    );
    assert_eq!(
        phase(
            "Read agent sees public notes",
            service.list_launch_profiles(agent, ListLaunchProfiles {})
        )
        .await
        .unwrap(),
        saved
    );
}

async fn check_reads(
    service: &Guarded,
    read: &Connection,
    admin: &Connection,
) -> LaunchProfilesSnapshot {
    let snapshot = phase(
        "Read sees actual empty list",
        service.list_launch_profiles(read, ListLaunchProfiles {}),
    )
    .await
    .unwrap();
    assert!(snapshot.profiles().as_slice().is_empty());
    assert_eq!(
        phase(
            "no Read denied",
            service.list_launch_profiles(admin, ListLaunchProfiles {})
        )
        .await
        .unwrap_err()
        .code,
        ErrorCode::Forbidden
    );
    snapshot
}

async fn check_editor_origins(service: &Guarded, contexts: [&Connection; 2]) {
    let selection = LaunchSelection::new(
        HarnessKind::ClaudeCode.into(),
        None,
        LaunchChoices::default(),
    );
    for context in contexts {
        assert_eq!(
            phase(
                "independent human probe restriction",
                service.launch_profile_options(
                    context,
                    GetLaunchProfileOptions {
                        selection: selection.clone()
                    }
                )
            )
            .await
            .unwrap_err()
            .code,
            ErrorCode::Forbidden
        );
    }
}

fn metadata_request(revision: LaunchProfileRevision) -> ReplaceLaunchProfiles {
    let profile = LaunchProfile::new(
        LaunchProfileId::random(),
        "Planning".parse().unwrap(),
        LaunchSelection::new(
            HarnessKind::ClaudeCode.into(),
            Some("saved-missing-account".parse().unwrap()),
            LaunchChoices::default(),
        ),
    )
    .with_notes(Some("Public usage guidance".parse().unwrap()));
    ReplaceLaunchProfiles {
        expected: revision,
        profiles: vec![profile].try_into().unwrap(),
    }
}
