//! Synthetic definitions and terminal observations are not launch fixtures.

use chrono::Utc;

use super::*;
use crate::workspace_services::{
    ServiceHealth, ServicePortPlan, ServiceProxyState, ServiceUnavailable,
};
use crate::{
    ProjectId, SessionStatus, TerminalId,
    terminals::{TerminalKind, TerminalPlace},
};

fn definition(kind: ScriptKind) -> ScriptEntry {
    ScriptEntry::new(
        "web".parse().unwrap(),
        kind,
        "new-command".parse().unwrap(),
        None,
        None,
    )
    .unwrap()
}

fn terminal() -> TerminalInfo {
    TerminalInfo::new(
        TerminalId::random(),
        TerminalPlace::Project {
            project: ProjectId::random(),
            dir: None,
        },
        "synthetic-checkout".into(),
        TerminalKind::Service {
            script: "web".parse().unwrap(),
            command: "old-command".parse().unwrap(),
        },
        Utc::now(),
    )
}

fn observation() -> ServiceObservation {
    ServiceObservation {
        port_plan: ServicePortPlan::Unplanned,
        health: ServiceHealth::NotObserved,
        proxy: ServiceProxyState::Unavailable {
            reason: ServiceUnavailable::ListenerUnavailable,
        },
    }
}

#[test]
fn plain_script_service_data_refuses_at_construction_and_wire_decode() {
    let entry = definition(ScriptKind::Script);
    assert_eq!(
        WorkspaceScriptRow::new(
            entry.clone(),
            WorkspaceScriptOrigin::Configured,
            None,
            Some(observation())
        ),
        Err(InvalidWorkspaceScriptRow::ServiceOnScript)
    );
    let text = format!(
        r#"{{"definition":{},"origin":"configured","service":{}}}"#,
        serde_json::to_string(&entry).unwrap(),
        serde_json::to_string(&observation()).unwrap()
    );
    assert!(serde_json::from_str::<WorkspaceScriptRow>(&text).is_err());
    let valid =
        WorkspaceScriptRow::new(entry, WorkspaceScriptOrigin::Configured, None, None).unwrap();
    let text = serde_json::to_string(&valid).unwrap();
    assert_eq!(
        serde_json::from_str::<WorkspaceScriptRow>(&text).unwrap(),
        valid
    );
}

#[test]
fn current_definition_and_prior_running_command_are_not_conflated() {
    let entry = definition(ScriptKind::Service);
    let running = terminal();
    let row = WorkspaceScriptRow::new(
        entry.clone(),
        WorkspaceScriptOrigin::Configured,
        Some(running.clone()),
        Some(observation()),
    )
    .unwrap();
    let WorkspaceScriptRowView {
        definition: displayed,
        origin,
        terminal,
        service,
    } = row.view();
    assert_eq!(displayed, &entry);
    assert_eq!(origin, WorkspaceScriptOrigin::Configured);
    assert_eq!(terminal, Some(&running));
    assert_eq!(service, Some(&observation()));
    let text = serde_json::to_string(&row).unwrap();
    assert_eq!(
        serde_json::from_str::<WorkspaceScriptRow>(&text).unwrap(),
        row
    );
    // The reference uses the current configured role even if the prior run
    // was a service. Its captured terminal observation remains explicit.
    let plain = WorkspaceScriptRow::new(
        definition(ScriptKind::Script),
        WorkspaceScriptOrigin::Configured,
        Some(running),
        None,
    )
    .unwrap();
    assert!(plain.view().service.is_none());
}

#[test]
fn stopped_removed_entries_and_unrelated_terminals_refuse() {
    let entry = definition(ScriptKind::Service);
    assert_eq!(
        WorkspaceScriptRow::new(
            entry.clone(),
            WorkspaceScriptOrigin::RemovedRunning,
            None,
            None
        ),
        Err(InvalidWorkspaceScriptRow::RemovedNotRunning)
    );
    let live = terminal();
    assert!(
        WorkspaceScriptRow::new(
            entry.clone(),
            WorkspaceScriptOrigin::RemovedRunning,
            Some(live.clone()),
            Some(observation())
        )
        .is_ok()
    );
    let mut stopped = live.clone();
    stopped.status = SessionStatus::Exited { code: Some(0) };
    let text = format!(
        r#"{{"definition":{},"origin":"removed_running","terminal":{}}}"#,
        serde_json::to_string(&entry).unwrap(),
        serde_json::to_string(&stopped).unwrap()
    );
    assert!(serde_json::from_str::<WorkspaceScriptRow>(&text).is_err());
    assert_eq!(
        WorkspaceScriptRow::new(
            entry.clone(),
            WorkspaceScriptOrigin::RemovedRunning,
            Some(stopped),
            None
        ),
        Err(InvalidWorkspaceScriptRow::RemovedNotRunning)
    );
    let mut foreign = live;
    foreign.kind = TerminalKind::Script {
        script: "other".parse().unwrap(),
        command: "old-command".parse().unwrap(),
    };
    assert_eq!(
        WorkspaceScriptRow::new(
            entry,
            WorkspaceScriptOrigin::Configured,
            Some(foreign),
            None
        ),
        Err(InvalidWorkspaceScriptRow::Terminal)
    );
}
