//! Separate in-process nodes keep identities and real store/runtime counts apart.
//! These are fake harness fixtures, not physical-machine or assigned-wire proof.
use super::{
    authority,
    deadline::{self, phase},
};
use crate::{Config, Node};
use std::path::PathBuf;
use vessel_harness::{
    HarnessEnv, Lookup,
    fakes::{Role, install},
};
use vessel_proto::{
    HarnessKind, SessionMode, SessionSpec,
    diagnostics::{DiagnosticReport, DiagnosticSection},
};
use vessel_store::DataDir;
use vessel_testkit::Fake;

fn live(report: &DiagnosticReport) -> u64 {
    let DiagnosticSection::Available { value, .. } = &report.fields().sessions else {
        panic!("runtime source unavailable")
    };
    value.live.get()
}
fn stored(report: &DiagnosticReport) -> u64 {
    let DiagnosticSection::Available { value, .. } = &report.fields().store else {
        panic!("store source unavailable")
    };
    value.sessions.total.get()
}
async fn start(root: &std::path::Path, lookup: Lookup) -> Node {
    let mut config = Config::new(
        DataDir::new(root.join("data")),
        HarnessEnv::new(root.join("user")),
    );
    config.bind = None;
    config.harness_lookup = lookup;
    phase("start per-node source fixture", Node::start(config))
        .await
        .unwrap()
}
#[tokio::test]
async fn two_node_reports_follow_their_own_runtime_store_and_admitted_identity() {
    deadline::run(async {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let bin = first.path().join("bin");
        let work = first.path().join("work");
        std::fs::create_dir(&bin).unwrap();
        std::fs::create_dir(&work).unwrap();
        let _fake = install(
            &bin,
            HarnessKind::ClaudeCode,
            Role::Structured,
            &Fake::Acp.path().unwrap(),
        )
        .unwrap();
        let a = start(first.path(), Lookup::new([bin])).await;
        let b = start(second.path(), Lookup::new(Vec::<PathBuf>::new())).await;
        let transport = authority::transport(&a).await;
        let session = phase(
            "create only on first node through the actual guard",
            transport.client().create_session(
                SessionSpec::new(
                    HarnessKind::ClaudeCode,
                    SessionMode::Structured {
                        model: None,
                        resume: None,
                        options: Vec::new(),
                    },
                )
                .with_cwd(work),
            ),
        )
        .await
        .unwrap();
        assert!(session.status.is_live());
        let report_a = phase(
            "collect first node sources",
            a.secure
                .service
                .0
                .diagnostics
                .collect(&a.secure.service.0.lifecycle),
        )
        .await
        .unwrap();
        let report_b = phase(
            "collect second node sources",
            b.secure
                .service
                .0
                .diagnostics
                .collect(&b.secure.service.0.lifecycle),
        )
        .await
        .unwrap();
        assert_ne!(report_a.fields().node.id, report_b.fields().node.id);
        assert_eq!(report_a.fields().node.id, a.info().id);
        assert_eq!(report_b.fields().node.id, b.info().id);
        assert_eq!(live(&report_a), 1);
        assert_eq!(stored(&report_a), 1);
        assert_eq!(live(&report_b), 0);
        assert_eq!(stored(&report_b), 0);
        transport.close().await;
        phase("stop first node with fake session", a.stop())
            .await
            .unwrap();
        phase("stop empty second node", b.stop()).await.unwrap();
    })
    .await;
}
