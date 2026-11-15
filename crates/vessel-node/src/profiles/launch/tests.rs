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

#[test]
fn launch_policy_is_frozen_and_uses_the_childs_exact_provider() {
    use vessel_proto::mcp::{
        Injection, McpSettings, ProviderPolicies, ProviderPolicy, StoredTool, ToolAccess, ToolId,
        ToolPolicy,
    };
    let home = tempfile::tempdir().unwrap();
    let enabled = McpSettings {
        inject: Injection::On,
        policy: ProviderPolicies::new(vec![ProviderPolicy {
            harness: HarnessKind::Codex.into(),
            policy: ToolPolicy::new(
                ToolAccess::Enabled,
                vec![StoredTool::Known(ToolId::CreateSession)],
            )
            .unwrap(),
        }])
        .unwrap(),
    };
    let (settings, source) = watch::channel(enabled);
    let launcher = Launcher::new(
        home.path().join("mcp"),
        Arc::new(NoMcpServers),
        Some(AgentTools {
            program: home.path().join("vessel"),
            args: vec!["mcp".to_owned()],
            socket: home.path().join("node.sock"),
        }),
    )
    .with_tool_policy(source);
    let codex = launcher.tools_for(&HarnessKind::Codex.into()).unwrap();
    let claude = launcher.tools_for(&HarnessKind::ClaudeCode.into()).unwrap();
    assert!(!codex.permits(ToolId::CreateSession));
    assert!(claude.permits(ToolId::CreateSession));
    let _previous = settings.send_replace(McpSettings::default());
    assert!(launcher.tools_for(&HarnessKind::Codex.into()).is_none());
    assert!(
        !codex.permits(ToolId::CreateSession),
        "an already prepared launch keeps its snapshot"
    );
    let mut servers = ResolvedMcpServers::new();
    launcher
        .attach_tools(
            &mut servers,
            SessionId::random(),
            &AgentSecret::from_bytes([5; 32]),
            &codex,
        )
        .unwrap();
    let server = servers.values().next().unwrap();
    let McpTransport::Stdio { env, .. } = server else {
        panic!("expected stdio");
    };
    let encoded = env.get(&TOOL_POLICY_VARIABLE.parse().unwrap()).unwrap();
    let policy: ToolPolicy = serde_json::from_str(encoded.expose()).unwrap();
    assert!(!policy.permits(ToolId::CreateSession));
}
