//! Real fake-driver probes and completed partial application, without any prompt.
use super::*;
use crate::{guard::Guarded, principal::Principal, service::Connection};
use vessel_proto::launch_profiles::*;
use vessel_proto::{ErrorCode, Handler as _, HarnessKind, Scope};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn selected_probe_and_live_bundle_use_actual_acp_choices_and_refusals() {
    vessel_testkit::within(
        SAFETY,
        Box::pin(async {
            let fixture = Fixture::new().await;
            let service = fixture.service();
            let owner = fixture.context(Principal::Owner).await;
            let read = fixture
                .context(Principal::Device {
                    id: vessel_proto::DeviceId::random(),
                    scopes: vec![Scope::Read],
                })
                .await;
            check_selected_probe(&service, &owner.connection, &read.connection).await;
            let handle = check_applicable(&fixture, &owner.connection, &read.connection).await;

            let request = ApplySessionOptions {
                session: handle.id(),
                options: bundle(&[
                    ("model", "fake-large"),
                    ("mode", "locked"),
                    ("effort", "low"),
                ]),
            };
            let agent = check_apply_denials(&fixture, &read.connection, &request).await;
            check_partial_application(&service, &owner.connection, &handle, request).await;
            agent.finish().await;
            read.finish().await;
            owner.finish().await;
            fixture.finish().await;
        }),
    )
    .await
    .unwrap();
}

async fn check_selected_probe(service: &Guarded, owner: &Connection, read: &Connection) {
    let selection = LaunchSelection::new(
        HarnessKind::ClaudeCode.into(),
        None,
        bundle(&[("model", "fake-large"), ("mode", "plan")]),
    );
    let catalogue = phase(
        "actual selected probe",
        service.launch_profile_options(
            read,
            GetLaunchProfileOptions {
                selection: selection.clone(),
            },
        ),
    )
    .await
    .unwrap();
    assert_eq!(catalogue.selection(), &selection);
    assert_eq!(catalogue.account().kind(), LaunchAccountKind::Default);
    assert!(
        catalogue
            .options()
            .as_slice()
            .iter()
            .any(|option| option.id.as_str() == "model"
                && option.current_name() == Some("Fake-large"))
    );
    assert!(
        phase(
            "probe makes no persistent session",
            service.list_sessions(owner, vessel_proto::ListSessions {})
        )
        .await
        .unwrap()
        .is_empty()
    );
    let missing = LaunchSelection::new(
        HarnessKind::ClaudeCode.into(),
        Some("missing".parse().unwrap()),
        LaunchChoices::default(),
    );
    assert_eq!(
        phase(
            "missing account never falls back",
            service.launch_profile_options(owner, GetLaunchProfileOptions { selection: missing })
        )
        .await
        .unwrap_err()
        .code,
        ErrorCode::InvalidRequest
    );
}

async fn check_applicable(
    fixture: &Fixture,
    owner: &Connection,
    read: &Connection,
) -> vessel_session::SessionHandle {
    let service = fixture.service();
    let preset: ReplaceLaunchProfiles =
        serde_json::from_str(vessel_testkit::launch_profile_fixtures::REPLACE).unwrap();
    let stored = phase(
        "save real fixture file",
        service.replace_launch_profiles(owner, preset),
    )
    .await
    .unwrap();
    let handle = fixture.running(owner).await;
    let id = handle.id();
    let eligible = phase(
        "node filters applicable account",
        service.applicable_launch_profiles(read, GetApplicableLaunchProfiles { session: id }),
    )
    .await
    .unwrap();
    assert_eq!(eligible.profiles(), stored.profiles());
    handle
}

async fn check_apply_denials(
    fixture: &Fixture,
    read: &Connection,
    request: &ApplySessionOptions,
) -> Context {
    let service = fixture.service();
    assert_eq!(
        phase(
            "Read cannot apply",
            service.apply_session_options(read, request.clone())
        )
        .await
        .unwrap_err()
        .code,
        ErrorCode::Forbidden
    );
    let agent = fixture
        .delegated_agent(vessel_proto::NodeId::random())
        .await;
    assert_eq!(
        phase(
            "unrelated agent cannot apply",
            service.apply_session_options(&agent.connection, request.clone())
        )
        .await
        .unwrap_err()
        .code,
        ErrorCode::Forbidden
    );
    agent
}

async fn check_partial_application(
    service: &Guarded,
    owner: &Connection,
    handle: &vessel_session::SessionHandle,
    request: ApplySessionOptions,
) {
    let id = handle.id();
    let result = phase(
        "actual ACP prefix and failure",
        service.apply_session_options(owner, request),
    )
    .await
    .unwrap();
    assert_eq!(
        result.applied().as_slice(),
        bundle(&[("model", "fake-large")]).as_slice()
    );
    assert_eq!(
        result.failure().unwrap().choice.as_choice().id.as_str(),
        "mode"
    );
    assert_eq!(result.failure().unwrap().error.code, ErrorCode::Conflict);
    let options = phase("read actual process options", handle.options_fenced())
        .await
        .unwrap();
    assert!(
        options
            .iter()
            .any(|option| option.id.as_str() == "model"
                && option.current_name() == Some("Fake-large"))
    );
    assert!(
        options
            .iter()
            .any(|option| option.id.as_str() == "effort" && option.current_name() == Some("High"))
    );
    let mut ending = handle.watch_info();
    phase("stop actual session", async {
        // kill acknowledges a request, not terminal publication. Observe the
        // real terminal event before testing the ended-session refusal.
        service.0.sessions.kill(id).await.unwrap();
        let _ended = ending
            .wait_for(|info| !info.status.is_live())
            .await
            .unwrap();
        assert!(handle.has_ended());
    })
    .await;
    assert!(
        phase(
            "ended process does not accept later bundle",
            service.apply_session_options(
                owner,
                ApplySessionOptions {
                    session: id,
                    options: bundle(&[("mode", "plan")])
                }
            )
        )
        .await
        .is_err()
    );
}
