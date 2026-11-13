//! Exact CLI output against the real node and the ACP usage fixture.
use crate::support::{Fixture, within};
use vessel_proto::usage::{QueryUsage, UsagePeriod, UsageScope};
use vessel_proto::{
    AgentEvent, ContentBlock, EventSeq, HarnessKind, SessionEvent, SessionInput, SessionMode,
    SessionSpec,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn usage_selectors_print_reported_counts_and_cumulative_session_cost() {
    within(async {
        let fixture = Fixture::start().await;
        let owner = fixture.owner().await;
        let id = owner.create_session(SessionSpec::new(HarnessKind::ClaudeCode, SessionMode::Structured { model: None, resume: None, options: Vec::new() }).with_cwd(fixture.data().to_owned())).await.unwrap().id;
        let mut stream = owner.follow(id, EventSeq::ZERO).await.unwrap();
        loop {
            let record = stream.next().await.expect("fake session ended before ready").unwrap();
            if matches!(record.event, SessionEvent::Status { status: vessel_proto::SessionStatus::Running }) { break; }
        }
        owner.send(id, SessionInput::Prompt { content: vec![ContentBlock::Text { text: "usage".to_owned() }] }).await.unwrap();
        while let Some(record) = stream.next().await {
            if matches!(record.unwrap().event, SessionEvent::Agent { event: AgentEvent::TurnEnded { .. } }) { break; }
        }
        let report = owner.usage(QueryUsage { scope: UsageScope::Session { session: id }, period: UsagePeriod::Session, after: None }).await.unwrap();
        let accounting = format!("Accounting began {}. Data kept for at most 90 days.\n", report.accounting_started.format("%Y-%m-%d %H:%M UTC"));
        let counts = "20 input, 10 output, 30 total tokens (reported turns: 1; unreported turns: 0)\n0.5 USD reported cost\n";
        let first = report.samples.first().unwrap().seq;
        let vessel_proto::usage::UsageSampleKind::Cost { since, .. } = report.samples.first().unwrap().kind else { panic!("first fixture sample is a cost increase"); };
        let since = since.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M %Z");
        let last = report.samples.last().unwrap().seq;
        let session = fixture.vessel(&["usage", "session", &id.to_string()], b"").await;
        assert!(session.ok, "{}", session.stderr);
        assert_eq!(session.stdout, format!("Retained session usage\n{counts}0.5 USD latest cumulative session cost\n{accounting}claude-code / default\n{counts}Report {first}: +0.5 USD since {since}\nReport {last}: 20 input, 10 output, 30 total tokens\n"));
        let machine = fixture.vessel(&["usage", "machine", "--period", "day"], b"").await;
        assert!(machine.ok, "{}", machine.stderr);
        assert_eq!(machine.stdout, format!("Past 24 hours\n{counts}{accounting}claude-code / default\n{counts}"));
        let agent = fixture.vessel(&["usage", "agent", "claude", "--period", "week"], b"").await;
        assert!(agent.ok, "{}", agent.stderr);
        assert_eq!(agent.stdout, format!("Past 7 days\n{counts}{accounting}claude-code / default\n{counts}"));
    }).await;
}
