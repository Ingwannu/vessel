//! The catalog and method calls against real directories and, on Unix, a
//! small `sh` helper that declares one method and answers its first call
//! (ADR 0115, "Catalog and invoke"). Nothing is fetched.

use vessel_proto::plugins::{
    HelperCall, InvokeContext, InvokePlugin, NodeToHelper, PluginGeneration, PluginStatus,
    RuntimeGeneration,
};
use vessel_proto::{ProjectId, RawJson, SessionId};

use super::super::tests::{Fixture, client_only, id, until};
use super::super::{CallFailure, PluginError};

/// A call to `plugin`'s `method` from the run `generation`, global context.
fn call(plugin: &str, generation: RuntimeGeneration, method: &str, input: RawJson) -> InvokePlugin {
    InvokePlugin {
        plugin: id(plugin),
        generation,
        method: method.parse().unwrap(),
        input,
        context: InvokeContext::Global,
    }
}

/// No input.
fn none() -> RawJson {
    RawJson::empty_object()
}

/// The envelope a helper's method receives, read back from the helper.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Received {
    input: serde_json::Value,
    context: InvokeContext,
}

/// One theme with Paseo's eight colors, as `vessel plugin init` writes.
const THEME_CLIENT: &str = r##"{"methods":[],"contributions":[],"themes":[{"id":"night","name":"Night dark","appearance":"dark","colors":{"background":"#15171c","foreground":"#e6e8ee","raised":"#1c1f26","control":"#252932","border":"#2f3440","accent":"#6ea8fe","mutedForeground":"#9aa3b2","ring":"#6ea8fe"}}]}"##;

#[tokio::test]
async fn the_catalog_holds_running_installations_at_the_watched_generation() {
    let fixture = Fixture::new();
    let plugins = fixture.host(false).await;
    let _directory = fixture.plugin(
        "night",
        &client_only("night"),
        &[("client.json", THEME_CLIENT.as_bytes())],
    );
    let _item = plugins.install("~/night", None, None).await.unwrap();
    // Plugins off: installed, not running, not in the catalog.
    let off = plugins.catalog(true).await.unwrap();
    assert!(off.catalog.installations().is_empty());
    plugins.set_switch(true);
    until(&plugins, "night", PluginStatus::Running).await;
    let on = plugins.catalog(true).await.unwrap();
    let [installation] = on.catalog.installations() else {
        panic!("one running installation: {:?}", on.catalog.installations());
    };
    assert_eq!(installation.address().plugin.as_str(), "night");
    assert_eq!(installation.document().themes().len(), 1);
    // The generation is the one `plugin.watch` counts.
    let current = plugins.watch(PluginGeneration::new(u64::MAX)).await;
    assert_eq!(on.generation, current);
    // A client-only plugin has no methods to call.
    let generation = installation.address().generation;
    let refused = plugins
        .invoke(call("night", generation, "night.read", none()))
        .await;
    assert!(
        matches!(refused, Err(PluginError::NoMethods(_))),
        "{refused:?}"
    );
    plugins.set_switch(false);
    until(&plugins, "night", PluginStatus::Disabled).await;
    let after = plugins.catalog(true).await.unwrap();
    assert!(after.catalog.installations().is_empty());
    fixture.finish().await;
}

#[cfg(unix)]
#[tokio::test]
async fn a_client_file_that_leads_outside_the_directory_does_not_start() {
    let fixture = Fixture::new();
    let plugins = fixture.host(true).await;
    let outside = fixture.home().join("outside.json");
    std::fs::create_dir_all(fixture.home()).unwrap();
    std::fs::write(&outside, THEME_CLIENT).unwrap();
    let directory = fixture.plugin("night", &client_only("night"), &[]);
    std::os::unix::fs::symlink(&outside, directory.join("client.json")).unwrap();
    let installed = plugins.install("~/night", None, None).await;
    assert!(
        matches!(installed, Err(PluginError::Failed { .. })),
        "{installed:?}"
    );
    let listed = plugins.list(true).await;
    let statuses: Vec<PluginStatus> = listed.plugins.iter().map(|item| item.status).collect();
    assert_eq!(statuses, [PluginStatus::Failed]);
    let catalog = plugins.catalog(true).await.unwrap();
    assert!(catalog.catalog.installations().is_empty());
    fixture.finish().await;
}

#[tokio::test]
async fn a_call_to_a_plugin_that_is_not_installed_or_not_running_is_refused() {
    let fixture = Fixture::new();
    let plugins = fixture.host(false).await;
    let any = RuntimeGeneration::random();
    let missing = plugins.invoke(call("nothing", any, "a.read", none())).await;
    assert!(
        matches!(missing, Err(PluginError::NotInstalled(_))),
        "{missing:?}"
    );
    let _directory = fixture.plugin(
        "night",
        &client_only("night"),
        &[("client.json", THEME_CLIENT.as_bytes())],
    );
    let _item = plugins.install("~/night", None, None).await.unwrap();
    let stopped = plugins.invoke(call("night", any, "a.read", none())).await;
    assert!(
        matches!(stopped, Err(PluginError::NotRunning(_))),
        "{stopped:?}"
    );
    fixture.finish().await;
}

#[cfg(unix)]
#[tokio::test]
async fn a_helper_answers_its_declared_method_and_refuses_an_undeclared_one() {
    let fixture = Fixture::new();
    let plugins = fixture.host(true).await;
    let kept = answering_plugin(&fixture, "echo");
    let _item = plugins.install("~/echo", None, None).await.unwrap();
    until(&plugins, "echo", PluginStatus::Running).await;
    let catalog = plugins.catalog(true).await.unwrap();
    let [installation] = catalog.catalog.installations() else {
        panic!("one running helper: {:?}", catalog.catalog.installations());
    };
    let declared: Vec<String> = installation
        .document()
        .methods()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(declared, ["echo.read"]);
    let generation = installation.address().generation;
    // A call from an older catalog is refused before the helper sees it.
    let older = plugins
        .invoke(call(
            "echo",
            RuntimeGeneration::random(),
            "echo.read",
            none(),
        ))
        .await;
    let Err(PluginError::Call { failure, .. }) = &older else {
        panic!("a call from another run is refused: {older:?}");
    };
    assert_eq!(*failure, CallFailure::Changed);
    let input = RawJson::from_text(r#"{"query":"메모"}"#).unwrap();
    // The node passes the context as the caller stated it (ADR 0115).
    let context = InvokeContext::Agent {
        project: ProjectId::random(),
        worktree: None,
        agent: SessionId::random(),
    };
    let output = plugins
        .invoke(InvokePlugin {
            context: context.clone(),
            ..call("echo", generation, "echo.read", input)
        })
        .await
        .unwrap();
    assert_eq!(output.as_text(), r#"{"ok":true}"#);
    // What the helper received: the method, and `{input, context}`.
    let request: NodeToHelper = serde_json::from_slice(&std::fs::read(&kept).unwrap()).unwrap();
    let NodeToHelper::Request {
        call: HelperCall::Invoke { method, input },
        ..
    } = request
    else {
        panic!("the helper's first request is the call: {request:?}");
    };
    assert_eq!(method.as_str(), "echo.read");
    let received: Received = serde_json::from_str(RawJson::from(input).as_text()).unwrap();
    assert_eq!(received.input, serde_json::json!({ "query": "메모" }));
    assert_eq!(received.context, context);
    let undeclared = plugins
        .invoke(call("echo", generation, "echo.write", none()))
        .await;
    let Err(PluginError::Call { failure, .. }) = &undeclared else {
        panic!("an undeclared method is refused: {undeclared:?}");
    };
    assert!(matches!(failure, CallFailure::Undeclared(_)), "{failure:?}");
    fixture.finish().await;
}

/// Between the owner's switch-off and its application the helper still
/// runs; neither the catalog nor a call reaches it then (node review
/// note 3).
#[cfg(unix)]
#[tokio::test]
async fn a_switched_off_helper_takes_no_call_before_its_stop() {
    let fixture = Fixture::new();
    let plugins = fixture.host(true).await;
    let _kept = answering_plugin(&fixture, "echo");
    let _item = plugins.install("~/echo", None, None).await.unwrap();
    until(&plugins, "echo", PluginStatus::Running).await;
    let catalog = plugins.catalog(true).await.unwrap();
    let [installation] = catalog.catalog.installations() else {
        panic!("one running helper: {:?}", catalog.catalog.installations());
    };
    let generation = installation.address().generation;
    // Another change holds the lock, so the switch-off waits to apply.
    let held = plugins.change().await.unwrap();
    plugins.set_switch(false);
    let refused = plugins
        .invoke(call("echo", generation, "echo.read", none()))
        .await;
    assert!(
        matches!(refused, Err(PluginError::NotRunning(_))),
        "{refused:?}"
    );
    let off = plugins.catalog(true).await.unwrap();
    assert!(off.catalog.installations().is_empty());
    drop(held);
    until(&plugins, "echo", PluginStatus::Disabled).await;
    fixture.finish().await;
}

/// A plugin whose `sh` helper declares `echo.read`, reads `initialize`,
/// then the first request (the node's first invocation id is 1), keeps that
/// request's frame in `request.json` beside it, answers it with
/// `{"ok":true}`, and reads frames until the node closes its pipe. Returns
/// where the request is kept.
#[cfg(unix)]
fn answering_plugin(fixture: &Fixture, name: &str) -> std::path::PathBuf {
    use std::fmt::Write as _;
    use std::num::NonZeroU64;

    use vessel_proto::plugins::{
        ClientDocument, HelperPayload, HelperProtocol, HelperToNode, HookSubscriptions,
        InvocationId,
    };

    let frame = |message: &HelperToNode| {
        let body = serde_json::to_vec(message).unwrap();
        let mut frame = u32::try_from(body.len()).unwrap().to_be_bytes().to_vec();
        frame.extend_from_slice(&body);
        let mut escaped = String::new();
        for byte in frame {
            write!(escaped, "\\{byte:03o}").unwrap();
        }
        escaped
    };
    let ready = frame(&HelperToNode::Ready {
        protocol: HelperProtocol::V1,
        client: ClientDocument::new(vec!["echo.read".parse().unwrap()], vec![], vec![]).unwrap(),
        hooks: HookSubscriptions::default(),
        settings: vessel_proto::plugins::SettingsDeclarations::default(),
    });
    let answer = frame(&HelperToNode::Result {
        request: InvocationId::new(NonZeroU64::MIN),
        output: HelperPayload::try_from(RawJson::from_text(r#"{"ok":true}"#).unwrap()).unwrap(),
    });
    let kept = fixture.home().join(name).join("request.json");
    let read_frame = |to: &str| {
        format!(
            "set -- $(dd bs=1 count=4 2>/dev/null | od -An -tu1)\n\
             [ \"$#\" -eq 4 ] || exit 0\n\
             length=$(( ($1 << 24) | ($2 << 16) | ($3 << 8) | $4 ))\n\
             dd bs=1 count=\"$length\" of='{to}' 2>/dev/null\n"
        )
    };
    let script = format!(
        "#!/bin/sh\nprintf '{ready}'\n{initialize}{request}\
         printf '{answer}'\nwhile :; do\n{rest}done\n",
        initialize = read_frame("/dev/null"),
        request = read_frame(&kept.display().to_string()),
        rest = read_frame("/dev/null"),
    );
    super::super::tests::helper_plugin(fixture, name, script.as_bytes());
    kept
}
