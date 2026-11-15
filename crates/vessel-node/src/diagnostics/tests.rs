//! Real node sources composed directly while the wire number is unassigned.
use super::{
    deadline::{self, phase},
    *,
};
use crate::{Config, Node};
use vessel_harness::HarnessEnv;
use vessel_proto::diagnostics::{MAX_DIAGNOSTIC_AGENTS, MAX_DIAGNOSTIC_REPORT_BYTES};
use vessel_store::DataDir;

const PRIVATE: &str = "PRIVATE-DIAGNOSTIC-NODE-CANARY";
#[tokio::test]
async fn real_node_reports_only_safe_sources_without_running_an_agent() {
    deadline::run(async {
        let temp = tempfile::tempdir().unwrap();
        let mut config = Config::new(
            DataDir::new(temp.path().join(PRIVATE)),
            HarnessEnv::new(temp.path().join("user")),
        );
        config.bind = None;
        config.name = Some(PRIVATE.to_owned());
        config.harness_lookup = Lookup::new(Vec::<std::path::PathBuf>::new());
        let node = phase("start real diagnostic node", Node::start(config))
            .await
            .unwrap();
        let state = &node.secure.service.0.diagnostics;
        let report = phase(
            "compose real source report",
            state.collect(&node.secure.service.0.lifecycle),
        )
        .await
        .unwrap();
        let fields = report.fields();
        assert_eq!(fields.node.id, node.info().id);
        assert_eq!(fields.node.home, DiagnosticHome::Redacted);
        assert!(fields.node.uptime.as_duration() > std::time::Duration::ZERO);
        let DiagnosticSection::Available { value, .. } = &fields.store else {
            panic!("store unavailable")
        };
        assert_eq!(value.expected_schema, value.observed_schema);
        assert_eq!(value.sessions.total.get(), 0);
        let DiagnosticSection::Available { value, .. } = &fields.agents else {
            panic!("agents unavailable")
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.len() <= MAX_DIAGNOSTIC_REPORT_BYTES);
        assert!(!json.contains(PRIVATE));
        assert!(!json.contains(temp.path().to_str().unwrap()));
        assert!(!json.contains(&node.info().public_key.unwrap().to_string()));
        assert!(serde_json::to_string(value).unwrap().len() < MAX_DIAGNOSTIC_REPORT_BYTES);
        assert!(MAX_DIAGNOSTIC_AGENTS >= vessel_proto::HarnessKind::ALL.len());
        assert!(matches!(
            fields.listeners,
            DiagnosticSection::Available {
                value: DiagnosticListeners {
                    local: DiagnosticBinding::Bound,
                    direct: DiagnosticBinding::Disabled,
                    ..
                },
                ..
            }
        ));
        phase("stop diagnostic node", node.stop()).await.unwrap();
    })
    .await;
}
