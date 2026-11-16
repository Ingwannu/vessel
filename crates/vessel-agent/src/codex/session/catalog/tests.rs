//! Actual correlated catalog replies must retain usable skills without logging native data.

mod capture;

use std::error::Error as _;
use std::time::Duration;

use serde_json::json;
use tokio::sync::mpsc;
use tracing::instrument::WithSubscriber as _;
use vessel_proto::{ContentBlock, ResolvedMcpServers};

use super::super::super::protocol::UserInput;
use super::super::super::wire::Outgoing;
use super::super::AgentSetup;
use super::*;

const CANARY: &str = "synthetic-private-native-value";
const OUTPUTS: usize = 4;
const HANDSHAKE: Duration = Duration::from_secs(60);
// A phase must fail before the unchanged case/handshake safety net masks it.
const CATALOG_PHASE: Duration = Duration::from_secs(20);

fn fixture() -> (
    Session,
    mpsc::Receiver<Vec<u8>>,
    mpsc::Receiver<AgentOutput>,
) {
    let (out, written) = Outgoing::detached();
    let (outputs, received) = mpsc::channel(OUTPUTS);
    let setup = AgentSetup {
        cwd: std::env::temp_dir(),
        model: None,
        resume: None,
        fork: None,
        options: Vec::new(),
        mcp_servers: ResolvedMcpServers::new(),
        handshake_timeout: HANDSHAKE,
        questions: false,
        system_prompt: None,
        private_dir: None,
    };
    (Session::new(out, outputs, setup), written, received)
}

async fn answer(
    session: &mut Session,
    written: &mut mpsc::Receiver<Vec<u8>>,
    reply: Result<Value, RpcError>,
) {
    assert!(
        tokio::time::timeout(CATALOG_PHASE, session.request_catalog())
            .await
            .expect("catalog request send must settle")
    );
    let frame = tokio::time::timeout(CATALOG_PHASE, written.recv())
        .await
        .expect("correlated catalog request must be written")
        .unwrap();
    let request: Value = serde_json::from_slice(&frame).unwrap();
    assert_eq!(request["method"], "skills/list");
    let id = request["id"].as_u64().unwrap();
    assert!(
        tokio::time::timeout(CATALOG_PHASE, session.answered(id, reply))
            .await
            .expect("actual correlated catalog handler must settle")
            .is_none()
    );
    assert!(!session.pending.contains_key(&id));
}

fn malformed() -> [Value; 2] {
    [
        json!({"data":[{"skills":[{"name":"known", "description":"ordinary", "path":"/synthetic/path", "enabled":CANARY}]}]}),
        json!({"data": CANARY}),
    ]
}

#[test]
fn concrete_decoder_evidence_is_retained_behind_an_input_free_chain() {
    for response in malformed() {
        let error = serde_json::from_value::<SkillsListResponse>(response).unwrap_err();
        assert!(error.to_string().contains(CANARY));
        let failure = DecodeFailure(error);
        assert_eq!(failure.0.classify(), serde_json::error::Category::Data);
        assert!(failure.0.to_string().contains(CANARY));
        assert!(failure.source().is_none());
        for rendered in [failure.to_string(), format!("{failure:?}")] {
            assert!(rendered.contains("Data"));
            assert!(!rendered.contains(CANARY));
        }
    }
}

#[tokio::test]
async fn correlated_handler_logs_safe_evidence_and_keeps_the_last_valid_catalog() {
    vessel_testkit::within(HANDSHAKE, async {
        let (mut session, mut written, mut outputs) = fixture();
        let valid = json!({"data":[{"skills":[{"name":"review", "description":"Review changes", "path":"/skills/review/SKILL.md", "enabled":true}]}]});
        answer(&mut session, &mut written, Ok(valid)).await;
        let AgentOutput::Commands(offered) = tokio::time::timeout(CATALOG_PHASE, outputs.recv())
            .await.expect("valid catalog output must arrive").unwrap() else {
            panic!("catalog output missing")
        };
        assert_eq!(offered.len(), 1);
        assert_eq!(offered[0].name.as_str(), "review");
        let capture = capture::Capture::default();
        for response in malformed() {
            answer(&mut session, &mut written, Ok(response))
                .with_subscriber(capture.clone())
                .await;
            assert_eq!(session.catalog.commands(), offered);
            assert!(outputs.try_recv().is_err());
        }
        let unavailable = RpcError {
            code: super::super::METHOD_NOT_FOUND,
            message: CANARY.to_owned(),
        };
        answer(&mut session, &mut written, Err(unavailable))
            .with_subscriber(capture.clone())
            .await;
        assert_eq!(session.catalog.commands(), offered);
        assert!(outputs.try_recv().is_err());
        let logs = capture.text();
        assert_eq!(logs.matches("catalog decode Data").count(), 2);
        assert!(logs.contains("-32601"));
        assert!(!logs.contains(CANARY));
        assert_eq!(
            session
                .catalog
                .input(&[ContentBlock::Text {
                    text: "/review changes".to_owned()
                }])
                .unwrap(),
            vec![
                UserInput::Skill {
                    name: "review".to_owned(),
                    path: "/skills/review/SKILL.md".to_owned()
                },
                UserInput::Text {
                    text: "$review changes".to_owned()
                },
            ]
        );
    }).await.expect("correlated catalog privacy case must settle");
}
