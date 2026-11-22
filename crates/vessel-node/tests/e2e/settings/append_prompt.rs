use super::*;
use vessel_client::CallError;
use vessel_proto::reload::ReloadSession;
use vessel_proto::settings::{
    APPEND_SYSTEM_PROMPT_PROTOCOL, AppendPromptTransport, AppendSystemPrompt, GetAppendSystemPrompt,
};
use vessel_proto::{EventSeq, HarnessKind, Scope, SessionMode, SessionSpec, SessionStatus};
use vessel_testkit::{Fake, Role, install};

use crate::agents::following;
use crate::support::ask;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_private_prompt_is_kept_by_admin_and_legacy_settings_stay_public() {
    within(async {
        let fixture = Fixture::new();
        let node = fixture.start().await;
        let owner = fixture.connect(&node).await;
        let raw = " \n Synthetic host instruction.\n ";
        let prompt = raw.parse::<AppendSystemPrompt>().unwrap();
        let patch = SetNodeSettings {
            append_system_prompt: Some(prompt.clone()),
            ..SetNodeSettings::default()
        };

        if APPEND_SYSTEM_PROMPT_PROTOCOL.is_none() {
            let original = owner.call(GetNodeSettings {}).await.unwrap();
            assert!(original.append_system_prompt_transports.is_none());
            assert!(matches!(
                owner.call(patch).await,
                Err(CallError::UnassignedProtocol { .. })
            ));
            assert!(matches!(
                owner.call(GetAppendSystemPrompt {}).await,
                Err(CallError::UnassignedProtocol { .. })
            ));
            assert_eq!(owner.call(GetNodeSettings {}).await.unwrap(), original);
            node.stop().await.unwrap();
            return;
        }

        let result = owner.call(patch).await.unwrap();
        assert_eq!(
            result.append_system_prompt_transports,
            Some(vec![AppendPromptTransport::CodexAppServer])
        );
        assert!(
            !serde_json::to_string(&result)
                .unwrap()
                .contains("Synthetic host instruction")
        );
        let reader = crate::side_questions::access::device(&owner, &node, vec![Scope::Read]).await;
        let public = reader.call(GetNodeSettings {}).await.unwrap();
        assert_eq!(public, result);
        assert_eq!(
            code(reader.call(GetAppendSystemPrompt {}).await),
            ErrorCode::Forbidden
        );
        assert_eq!(owner.call(GetAppendSystemPrompt {}).await.unwrap(), prompt);
        owner
            .call(SetNodeSettings {
                park_after: Some(ParkAfter::Never),
                ..SetNodeSettings::default()
            })
            .await
            .unwrap();
        node.stop().await.unwrap();

        let node = fixture.start().await;
        let owner = fixture.connect(&node).await;
        let saved = owner.call(GetAppendSystemPrompt {}).await.unwrap();
        assert_eq!(serde_json::to_value(saved).unwrap(), raw);
        owner
            .call(SetNodeSettings {
                append_system_prompt: Some(AppendSystemPrompt::default()),
                ..SetNodeSettings::default()
            })
            .await
            .unwrap();
        assert_eq!(
            owner
                .call(GetAppendSystemPrompt {})
                .await
                .unwrap()
                .effective(),
            ""
        );
        assert_eq!(
            owner.call(GetNodeSettings {}).await.unwrap().park_after,
            ParkAfter::Never
        );
        node.stop().await.unwrap();

        verify_native_launches(&fixture, prompt).await;
    })
    .await;
}

async fn verify_native_launches(fixture: &Fixture, prompt: AppendSystemPrompt) {
    let fake = Fake::CodexAppServer.path().unwrap();
    drop(install(fixture.bin(), HarnessKind::Codex, Role::Terminal, &fake).unwrap());
    std::fs::write(
        fixture.data_dir().profiles_path(),
        r#"
[profiles.prompt-proof]
harness = "codex"
env = { FAKE_CODEX_EXPECT_DEVELOPER_INSTRUCTIONS_FILE = ".expected-instructions" }
"#,
    )
    .unwrap();
    let expected_path = fixture.cwd().join(".expected-instructions");
    std::fs::write(&expected_path, prompt.effective()).unwrap();

    let node = fixture.start().await;
    let owner = fixture.connect(&node).await;
    owner
        .call(SetNodeSettings {
            drivers: Some(SetAgentDrivers {
                codex: Some(AgentDriver::Direct),
                ..SetAgentDrivers::default()
            }),
            append_system_prompt: Some(prompt),
            ..SetNodeSettings::default()
        })
        .await
        .unwrap();

    let mut spec = SessionSpec::new(
        HarnessKind::Codex,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    )
    .with_cwd(fixture.cwd());
    spec.profile = Some("prompt-proof".parse().unwrap());
    let id = owner.create_session(spec).await.unwrap().id;
    let mut running = following(&owner, id, EventSeq::ZERO).await;
    let said = ask(&owner, &mut running.stream, &mut running.seen, id, "plain").await;
    assert_eq!(said, "fake reply: plain");

    // A blank host value must reach the same conversation on the next launch.
    owner
        .call(SetNodeSettings {
            append_system_prompt: Some(AppendSystemPrompt::default()),
            ..SetNodeSettings::default()
        })
        .await
        .unwrap();
    std::fs::write(expected_path, "").unwrap();
    let info = owner.call(ReloadSession { session: id }).await.unwrap();
    assert_eq!(info.status, SessionStatus::Running);
    let said = ask(&owner, &mut running.stream, &mut running.seen, id, "again").await;
    assert_eq!(said, "fake reply: again");
    node.stop().await.unwrap();
}
