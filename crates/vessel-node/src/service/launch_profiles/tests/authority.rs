//! Scope, origin and durable CAS exercised through the real generated guard.
use super::*;
use crate::principal::Principal;
use vessel_proto::launch_profiles::*;
use vessel_proto::{
    DeviceId, ErrorCode, Handler as _, HarnessKind, LaunchProfileId, Origin, OriginBy, Scope,
};

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
                .context(Principal::delegated(
                    DeviceId::random(),
                    &Scope::ALL,
                    Origin {
                        node: fixture.node.as_ref().unwrap().info().id,
                        by: OriginBy::Agent {
                            session: vessel_proto::SessionId::random(),
                            depth: 0,
                        },
                        scopes: Scope::ALL.to_vec(),
                    },
                    vessel_proto::NodeId::random(),
                ))
                .await;
            let snapshot = phase(
                "Read sees actual empty list",
                service.list_launch_profiles(&read.connection, ListLaunchProfiles {}),
            )
            .await
            .unwrap();
            assert!(snapshot.profiles().as_slice().is_empty());
            assert_eq!(
                phase(
                    "no Read denied",
                    service.list_launch_profiles(&no_read.connection, ListLaunchProfiles {})
                )
                .await
                .unwrap_err()
                .code,
                ErrorCode::Forbidden
            );
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
            let request = ReplaceLaunchProfiles {
                expected: snapshot.revision(),
                profiles: vec![profile].try_into().unwrap(),
            };
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
            let saved = phase(
                "owner saves missing account metadata",
                service.replace_launch_profiles(&owner.connection, request.clone()),
            )
            .await
            .unwrap();
            assert_eq!(saved.profiles(), &request.profiles);
            let same = phase(
                "Admin-only device may replace known revision",
                service.replace_launch_profiles(
                    &no_read.connection,
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
                    service.replace_launch_profiles(&owner.connection, request)
                )
                .await
                .unwrap_err()
                .code,
                ErrorCode::Conflict
            );
            assert_eq!(
                phase(
                    "Read agent sees public notes",
                    service.list_launch_profiles(&agent.connection, ListLaunchProfiles {})
                )
                .await
                .unwrap(),
                saved
            );
            let selection = LaunchSelection::new(
                HarnessKind::ClaudeCode.into(),
                None,
                LaunchChoices::default(),
            );
            for context in [&agent.connection, &delegated.connection] {
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
