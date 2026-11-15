//! Canonical fixtures pin distinctions a generic tool-output model would lose.

use serde_json::json;

use super::*;

#[test]
fn all_canonical_body_kinds_decode_without_provider_tool_names() {
    let fixtures = [
        json!({"type":"shell","command":"printf hi","cwd":"/workspace","output":"hi","exit_code":0}),
        json!({"type":"read","file_path":"a.rs","content":"x","offset":0,"limit":2}),
        json!({"type":"edit","file_path":"a.rs","old_string":"x","new_string":"","unified_diff":"-x"}),
        json!({"type":"write","file_path":"new.rs","content":"new"}),
        json!({"type":"search","query":"needle","tool_name":"grep","content":"1:needle","file_paths":["a.rs"],"web_results":[{"title":"Result","url":"https://example.org/"}],"annotations":["partial"],"num_files":8,"num_matches":9,"duration":50,"truncated":"truncated","mode":"content"}),
        json!({"type":"fetch","url":"https://example.org/","prompt":"extract","result":"body","code":200,"code_text":"OK","bytes":4,"duration":7}),
        json!({"type":"worktree_setup","worktree_path":"/workspace","branch_name":"topic/test","log":"setup","commands":[{"index":1,"command":"build","cwd":"/workspace","log":"started","status":"running","exit_code":null}],"truncated":"complete"}),
        json!({"type":"sub_agent","sub_agent_type":"review","description":"check","child_session_id":"provider-child","actions":[{"index":1,"tool_name":"read","summary":"looked"}]}),
        json!({"type":"plain_text","label":"Think","text":"reasoning","icon":"brain"}),
        json!({"type":"plan","text":"# Proposed\n- Step"}),
        json!({"type":"unknown","input":{"future":{"nested":true}},"output":[1,"opaque"]}),
    ];
    for fixture in fixtures {
        let detail: ToolCallDetail = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(detail).unwrap(), fixture);
    }
}

#[test]
fn body_validation_keeps_unknown_content_opaque_without_accepting_bad_identities() {
    let wrong_ordinal = json!({"type":"sub_agent","actions":[{"index":0,"tool_name":"read"}]});
    assert!(serde_json::from_value::<ToolCallDetail>(wrong_ordinal).is_err());
    let bad_url = json!({"type":"fetch","url":"https://user:pass@example.org/"});
    assert!(serde_json::from_value::<ToolCallDetail>(bad_url).is_err());
    let unknown = json!({"type":"unknown","input":{"extension":"new"}});
    let parsed: ToolCallDetail = serde_json::from_value(unknown.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), unknown);
}
