//! Policy invariants hold on constructors and serialized ingress alike.

use super::*;
use serde_json::json;

#[test]
fn an_exact_denylist_and_the_global_provider_switch_are_distinct() {
    let policy: ToolPolicy = serde_json::from_value(json!({"disabled":["get_session"]})).unwrap();
    assert!(!policy.permits(ToolId::Added(AddedId::GetSession)));
    assert!(policy.permits(ToolId::CreateSession));
    let off: ToolPolicy = serde_json::from_value(json!({"enabled":false})).unwrap();
    assert!(!off.permits(ToolId::CreateSession));
    assert_eq!(
        serde_json::from_str::<Injection>("false").unwrap(),
        Injection::Off
    );
    assert_eq!(serde_json::to_string(&Injection::On).unwrap(), "true");
}

#[test]
fn a_future_stored_name_roundtrips_but_cannot_be_newly_set() {
    let raw = json!({"enabled":true,"disabled":["future_tool"]});
    let policy: ToolPolicy = serde_json::from_value(raw.clone()).unwrap();
    assert_eq!(serde_json::to_value(&policy).unwrap(), raw);
    assert_eq!(policy.validate_edit(), Err(InvalidMcp::UnknownTool));
    assert!(policy.permits(ToolId::CreateSession));
}

#[test]
fn duplicate_or_excessive_policy_entries_are_refused() {
    assert!(
        serde_json::from_value::<ToolPolicy>(json!({"disabled":["get_session","get_session"]}))
            .is_err()
    );
    let excess = vec!["get_session"; MAX_DISABLED_TOOLS.saturating_add(1)];
    assert!(serde_json::from_value::<ToolPolicy>(json!({"disabled":excess})).is_err());
    let provider = json!({"harness":"codex"});
    assert!(serde_json::from_value::<ProviderPolicies>(json!([provider, provider])).is_err());
}

#[test]
fn custom_providers_do_not_inherit_a_base_provider_policy() {
    let policies: ProviderPolicies = serde_json::from_value(json!([
        {"harness":"codex","enabled":false}
    ]))
    .unwrap();
    let codex = "codex".parse().unwrap();
    let added = "custom-agent".parse().unwrap();
    assert!(!policies.for_harness(&codex).permits(ToolId::CreateSession));
    assert!(policies.for_harness(&added).permits(ToolId::CreateSession));
}

#[test]
fn invalid_future_identifiers_cannot_be_stored() {
    for text in ["", "../escape", "with space", "line\n", "한글"] {
        assert!(text.parse::<StoredTool>().is_err());
    }
    assert!(
        "x".repeat(MAX_TOOL_NAME_BYTES)
            .parse::<StoredTool>()
            .is_ok()
    );
    assert!(
        "x".repeat(MAX_TOOL_NAME_BYTES.saturating_add(1))
            .parse::<StoredTool>()
            .is_err()
    );
}

#[test]
fn browser_policy_uses_the_producers_one_closed_catalogue() {
    use crate::browser::{BROWSER_TOOLS, BrowserTool};
    for &tool in BROWSER_TOOLS {
        let id = ToolId::Browser(tool);
        assert_eq!(ToolId::parse(tool.name()), Some(id));
        assert_eq!(id.as_str(), tool.name());
    }
    let click = ToolId::Browser(BrowserTool::Click);
    let policy = ToolPolicy::new(ToolAccess::Enabled, vec![StoredTool::Known(click)]).unwrap();
    assert!(!policy.permits(click));
    assert!(policy.permits(ToolId::Browser(BrowserTool::Type)));
    let encoded = serde_json::to_value(&policy).unwrap();
    assert_eq!(encoded["disabled"][0], "browser_click");
    assert_eq!(
        serde_json::from_value::<ToolPolicy>(encoded).unwrap(),
        policy
    );
}
