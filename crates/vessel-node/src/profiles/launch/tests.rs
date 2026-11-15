//! Tests of a session's launch: profiles, MCP servers, the `vessel`
//! server and whether its agent may ask questions.

use std::collections::BTreeMap;

use vessel_proto::{HarnessKind, McpTransport, NodeId, ResolvedValue, SessionMode, TerminalSize};

use super::*;

/// One server, and one secret it could not find.
#[derive(Debug)]
struct OneServer;

impl McpSource for OneServer {
    fn secret(&self, _name: &SecretName) -> Option<ResolvedValue> {
        None
    }

    fn servers(&self, spec: &SessionSpec, _only: Option<&[McpServerName]>) -> McpServers {
        let mut servers = ResolvedMcpServers::new();
        let _none = servers.insert(
            "docs".parse().unwrap(),
            McpTransport::Stdio {
                command: PathBuf::from("docs-mcp"),
                args: vec![spec.harness.to_string()],
                env: BTreeMap::from([(
                    "DOCS_TOKEN".parse().unwrap(),
                    ResolvedValue::new("value-never-logged".to_owned()),
                )]),
            },
        );
        McpServers {
            servers,
            missing: vec!["WEB_TOKEN".parse().unwrap()],
        }
    }
}

#[test]
fn a_launch_gets_the_sources_servers_and_a_line_per_missing_secret() {
    let dir = tempfile::tempdir().unwrap();
    let profiles = NodeProfiles::load(
        vessel_store::DataDir::new(dir.path()),
        vessel_harness::HarnessEnv::new(dir.path().to_owned()),
    )
    .unwrap();
    let launcher = Launcher::new(dir.path().join("mcp"), Arc::new(OneServer), None);
    let spec = SessionSpec::new(
        HarnessKind::ClaudeCode,
        SessionMode::Terminal {
            args: Vec::new(),
            size: TerminalSize::default(),
        },
    );
    let options = launcher
        .options(&profiles, &spec, &Principal::Owner)
        .unwrap();
    let names: Vec<String> = options
        .mcp_servers
        .keys()
        .map(ToString::to_string)
        .collect();
    assert_eq!(names, ["docs"]);
    assert_eq!(options.notices.len(), 1);
    let notice = options.notices.first().unwrap();
    assert!(notice.starts_with("vessel: secret WEB_TOKEN "), "{notice}");
    assert!(!notice.contains("value-never-logged"), "{notice}");

    let none = Launcher::new(dir.path().join("mcp"), Arc::new(NoMcpServers), None)
        .options(&profiles, &spec, &Principal::Owner)
        .unwrap();
    assert!(none.mcp_servers.is_empty() && none.notices.is_empty());
    assert!(options.questions, "the owner watches its sessions");
}

/// Questions are offered where a person watches: the owner, a device,
/// either through another node; never an agent's session, here or
/// delivered.
#[test]
fn only_a_watched_session_asks() {
    let node = NodeId::random();
    let device = vessel_proto::DeviceId::random();
    let session = SessionId::random();
    let scopes = vessel_proto::Scope::ALL.to_vec();
    let asks = |principal: &Principal| watched(&creator(principal));
    let delivered = |by| Principal::Delegated {
        device,
        scopes: scopes.clone(),
        origin: vessel_proto::Origin {
            node,
            by,
            scopes: scopes.clone(),
        },
        via: node,
    };
    assert!(asks(&Principal::Owner));
    assert!(asks(&Principal::Device {
        id: device,
        scopes: scopes.clone(),
    }));
    assert!(asks(&delivered(OriginBy::Owner)));
    assert!(asks(&delivered(OriginBy::Device { id: device })));
    assert!(!asks(&Principal::Agent {
        node,
        session,
        scopes: scopes.clone(),
    }));
    assert!(!asks(&delivered(OriginBy::Agent { session, depth: 1 })));
}
