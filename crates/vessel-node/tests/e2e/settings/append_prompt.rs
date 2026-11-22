use super::*;
use vessel_client::CallError;
use vessel_proto::Scope;
use vessel_proto::settings::{
    APPEND_SYSTEM_PROMPT_PROTOCOL, AppendPromptTransport, AppendSystemPrompt, GetAppendSystemPrompt,
};

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
            assert!(matches!(owner.call(patch).await, Err(CallError::UnassignedProtocol { .. })));
            assert!(matches!(owner.call(GetAppendSystemPrompt {}).await, Err(CallError::UnassignedProtocol { .. })));
            assert_eq!(owner.call(GetNodeSettings {}).await.unwrap(), original);
            node.stop().await.unwrap();
            return;
        }

        let result = owner.call(patch).await.unwrap();
        assert_eq!(result.append_system_prompt_transports, Some(vec![AppendPromptTransport::CodexAppServer]));
        assert!(!serde_json::to_string(&result).unwrap().contains("Synthetic host instruction"));
        let reader = crate::side_questions::access::device(&owner, &node, vec![Scope::Read]).await;
        let public = reader.call(GetNodeSettings {}).await.unwrap();
        assert_eq!(public, result);
        assert_eq!(code(reader.call(GetAppendSystemPrompt {}).await), ErrorCode::Forbidden);
        assert_eq!(owner.call(GetAppendSystemPrompt {}).await.unwrap(), prompt);
        owner.call(SetNodeSettings {
            park_after: Some(ParkAfter::Never),
            ..SetNodeSettings::default()
        }).await.unwrap();
        node.stop().await.unwrap();

        let node = fixture.start().await;
        let owner = fixture.connect(&node).await;
        let saved = owner.call(GetAppendSystemPrompt {}).await.unwrap();
        assert_eq!(serde_json::to_value(saved).unwrap(), raw);
        owner.call(SetNodeSettings {
            append_system_prompt: Some(AppendSystemPrompt::default()),
            ..SetNodeSettings::default()
        }).await.unwrap();
        assert_eq!(owner.call(GetAppendSystemPrompt {}).await.unwrap().effective(), "");
        assert_eq!(owner.call(GetNodeSettings {}).await.unwrap().park_after, ParkAfter::Never);
        node.stop().await.unwrap();
    }).await;
}
