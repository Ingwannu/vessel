//! Reference-shaped arguments stay at the MCP edge; fixtures grant no browser access.
use super::*;
use serde_json::json;
use std::time::Duration;
use vessel_proto::browser::{
    BROWSER_TOOLS, COMMAND_TIMEOUT, DEFAULT_LOG_ENTRIES, MAX_VIEWPORT_SIDE, MAX_WAIT_MS,
};
const ID: &str = "00000000-0000-4000-8000-000000000001";

fn args(value: Value) -> JsonObject {
    let Value::Object(value) = value else {
        panic!("fixture must be an object")
    };
    value
}
fn fixture(tool: BrowserTool) -> JsonObject {
    let mut input = args(
        json!({"browserId":ID,"ref":"@e01","text":"typed","key":"Enter","value":"","function":"()=>2","sourceRef":"@e1","targetRef":"@e2","filePaths":["relative.txt"],"deltaX":0.5,"deltaY":-12,"width":390,"height":844}),
    );
    if !matches!(tool, BrowserTool::Wait) {
        let _replaced = input.insert("url".into(), json!(" example.test/path "));
    }
    input
}
#[test]
fn every_catalogued_tool_decodes_reference_fields_without_a_second_name_table() {
    for tool in BROWSER_TOOLS {
        let command = decode(*tool, &fixture(*tool)).unwrap();
        assert_eq!(command.tool(), *tool);
        assert_eq!(BrowserTool::from_name(tool.name()), Some(*tool));
        if let Some(target) = command.target() {
            assert_eq!(target.to_string(), ID);
        }
    }
}
#[test]
fn omitted_defaults_remain_distinct_from_explicit_wait_and_empty_text_is_valid() {
    let click = decode(
        BrowserTool::Click,
        &args(json!({"browserId":ID,"ref":"@e01"})),
    )
    .unwrap();
    let BrowserCommand::Click(click) = click else {
        panic!()
    };
    assert_eq!(click.button, MouseButton::Left);
    assert_eq!(click.count, ClickCount::Single);
    assert_eq!(click.modifiers, KeyModifiers::default());
    assert_eq!(click.target.element.to_string(), "@e01");
    let mut input = args(json!({"browserId":ID,"url":"partial-location"}));
    let omitted = decode(BrowserTool::Wait, &input).unwrap();
    assert_eq!(omitted.command_timeout(), COMMAND_TIMEOUT);
    let _replaced = input.insert("timeoutMs".into(), json!(MAX_WAIT_MS));
    let explicit = decode(BrowserTool::Wait, &input).unwrap();
    assert_eq!(
        explicit.command_timeout(),
        Duration::from_millis(u64::from(MAX_WAIT_MS)).saturating_add(Duration::from_secs(1))
    );
    let BrowserCommand::Wait(wait) = explicit else {
        panic!()
    };
    assert!(matches!(wait.condition, WaitFor::Url(_)));
    let fill = decode(
        BrowserTool::Fill,
        &args(json!({"browserId":ID,"ref":"@e1","value":""})),
    )
    .unwrap();
    let BrowserCommand::Fill(fill) = fill else {
        panic!()
    };
    assert_eq!(fill.value.as_str(), "");
    let logs = decode(BrowserTool::Logs, &args(json!({"browserId":ID}))).unwrap();
    let BrowserCommand::Logs(logs) = logs else {
        panic!()
    };
    assert_eq!(logs.count.get(), DEFAULT_LOG_ENTRIES);
}
#[test]
fn bad_fields_never_echo_secrets_or_create_caller_controlled_scope() {
    for (tool, input) in [
        (
            BrowserTool::Click,
            json!({"browserId":ID,"ref":"secret-ref"}),
        ),
        (
            BrowserTool::NewTab,
            json!({"url":"https://private-user:secret@example.test/"}),
        ),
        (
            BrowserTool::Wait,
            json!({"browserId":ID,"text":"a","url":"b"}),
        ),
        (BrowserTool::Wait, json!({"browserId":ID,"text":""})),
        (
            BrowserTool::Wait,
            json!({"browserId":ID,"url":"a","timeoutMs":0}),
        ),
        (
            BrowserTool::Resize,
            json!({"browserId":ID,"width":MAX_VIEWPORT_SIDE.saturating_add(1),"height":1}),
        ),
        (
            BrowserTool::Upload,
            json!({"browserId":ID,"ref":"@e1","filePaths":[]}),
        ),
        (BrowserTool::Type, json!({"browserId":ID,"text":null})),
        (
            BrowserTool::Click,
            json!({"browserId":ID,"ref":"@e1","modifiers":["secret-modifier"]}),
        ),
        (
            BrowserTool::Scroll,
            json!({"browserId":ID,"deltaX":"secret-delta","deltaY":0}),
        ),
    ] {
        let error = decode(tool, &args(input)).unwrap_err();
        assert!(!format!("{error:?} {error}").contains("secret"));
    }
    let command=decode(BrowserTool::ListTabs,&args(json!({"node":"foreign-node","workspace":"foreign-workspace","session":"foreign-session"}))).unwrap();
    assert_eq!(command, BrowserCommand::ListTabs(NoBrowserArgs {}));
}

#[test]
fn schemas_use_the_catalogue_and_express_the_wait_exclusion_without_offering_tools() {
    for tool in BROWSER_TOOLS {
        let built = schema::schema(*tool);
        assert_eq!(built.name.as_ref(), tool.name());
        assert_eq!(built.input_schema.get("type"), Some(&json!("object")));
    }
    let wait = schema::schema(BrowserTool::Wait);
    assert!(wait.input_schema.contains_key("oneOf"));
    let resize = schema::schema(BrowserTool::Resize);
    assert_eq!(
        resize.input_schema["properties"]["width"]["maximum"],
        json!(MAX_VIEWPORT_SIDE)
    );
}
