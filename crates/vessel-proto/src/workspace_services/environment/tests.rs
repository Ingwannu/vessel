//! Cached projection fixtures never allocate, probe or launch a real service.

use super::*;
use crate::scripts::ServiceEnvName;

fn peer(name: &str) -> ServiceEnvPeer {
    ServiceEnvPeer::new(name.parse().unwrap(), ServicePort::MIN, None)
}

fn selected_service() -> WorkspaceScriptRef {
    WorkspaceScriptRef {
        workspace: WorkspaceId::random(),
        script: "web-0".parse().unwrap(),
    }
}

fn ready(peers: Vec<ServiceEnvPeer>) -> Result<ReadyServiceEnv, InvalidReadyServiceEnv> {
    ReadyServiceEnv::new(
        selected_service(),
        ServiceSnapshotId::random(),
        ServiceBindHost::Ipv4Loopback,
        ServicePort::MIN,
        None,
        peers,
    )
}

#[test]
fn canonical_peer_keys_are_derived_not_independently_accepted() {
    let value = peer("web-app");
    assert_eq!(
        value.view().env_name,
        &ServiceEnvName::of(&"web-app".parse().unwrap())
    );
    let json = serde_json::to_string(&value).unwrap();
    assert!(json.contains("WEB_APP"));
    assert_eq!(
        serde_json::from_str::<ServiceEnvPeer>(&json).unwrap(),
        value
    );
    let wrong = format!(
        r#"{{"script":"web-app","env_name":"OTHER","port":{}}}"#,
        ServicePort::MIN
    );
    assert!(serde_json::from_str::<ServiceEnvPeer>(&wrong).is_err());
}

#[test]
fn committed_plan_count_collisions_and_selected_service_are_consistent() {
    let peers: Vec<_> = (0..MAX_PROJECT_COMMANDS)
        .map(|index| peer(&format!("web-{index}")))
        .collect();
    let value = ready(peers.clone()).unwrap();
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(
        serde_json::from_str::<ReadyServiceEnv>(&json).unwrap(),
        value
    );
    let mut oversized = peers;
    oversized.push(peer("extra"));
    assert_eq!(ready(oversized), Err(InvalidReadyServiceEnv::TooMany));
    assert_eq!(ready(vec![]), Err(InvalidReadyServiceEnv::MissingSelf));
    assert_eq!(
        ready(vec![peer("web-0"), peer("web-0")]),
        Err(InvalidReadyServiceEnv::Duplicate)
    );
    assert_eq!(
        ready(vec![peer("web-0"), peer("web-app"), peer("web_app")]),
        Err(InvalidReadyServiceEnv::Duplicate)
    );
    let ReadyServiceEnvView {
        workspace,
        snapshot,
        script,
        host,
        port,
        url,
        peers,
    } = value.view();
    assert_eq!(host.as_str(), "127.0.0.1");
    assert_eq!(port, ServicePort::MIN);
    assert!(url.is_none());
    assert_eq!(peers.len(), MAX_PROJECT_COMMANDS);
    let wrong = ReadyServiceEnv::new(
        WorkspaceScriptRef {
            workspace,
            script: script.clone(),
        },
        snapshot,
        host,
        ServicePort::MAX,
        None,
        vec![peer("web-0")],
    );
    assert_eq!(wrong, Err(InvalidReadyServiceEnv::SelfMismatch));
}

#[test]
fn decoder_cannot_invent_ready_peers_or_inherited_environment_fields() {
    let value = ready(vec![peer("web-0")]).unwrap();
    let view = value.view();
    for peers in [vec![], vec![peer("web-0"), peer("web-0")]] {
        let json = format!(
            r#"{{"workspace":"{}","snapshot":"{}","script":"web-0","host":"ipv4_loopback","port":{},"peers":{}}}"#,
            view.workspace,
            view.snapshot,
            ServicePort::MIN,
            serde_json::to_string(&peers).unwrap()
        );
        assert!(serde_json::from_str::<ReadyServiceEnv>(&json).is_err());
    }
    let oversized: Vec<_> = (0..MAX_PROJECT_COMMANDS.checked_add(1).unwrap())
        .map(|index| peer(&format!("web-{index}")))
        .collect();
    let json = format!(
        r#"{{"workspace":"{}","snapshot":"{}","script":"web-0","host":"ipv4_loopback","port":{},"peers":{}}}"#,
        view.workspace,
        view.snapshot,
        ServicePort::MIN,
        serde_json::to_string(&oversized).unwrap()
    );
    assert!(serde_json::from_str::<ReadyServiceEnv>(&json).is_err());
    let json = format!(
        r#"{{"workspace":"{}","snapshot":"{}","script":"web-0","host":"ipv4_loopback","port":{},"peers":[{}],"environment":{{"SYNTHETIC_CREDENTIAL":"canary"}}}}"#,
        view.workspace,
        view.snapshot,
        ServicePort::MIN,
        serde_json::to_string(&peer("web-0")).unwrap()
    );
    assert!(serde_json::from_str::<ReadyServiceEnv>(&json).is_err());
    assert_eq!(
        serde_json::from_str::<ServiceEnvPlan>(r#"{"kind":"future"}"#).unwrap(),
        ServiceEnvPlan::Unknown
    );
}
