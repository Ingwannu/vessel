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
        for prompt in ["usage-reasoning", "usage"] {
            owner.send(id, SessionInput::Prompt { content: vec![ContentBlock::Text { text: prompt.to_owned() }] }).await.unwrap();
            while let Some(record) = stream.next().await {
                if matches!(record.unwrap().event, SessionEvent::Agent { event: AgentEvent::TurnEnded { .. } }) { break; }
            }
        }
        let report = owner.usage(QueryUsage { scope: UsageScope::Session { session: id }, period: UsagePeriod::Session, after: None }).await.unwrap();
        let accounting = format!("Accounting began {}. Data kept for at most 90 days.\n", report.accounting_started.format("%Y-%m-%d %H:%M UTC"));
        let counts = "40 input, 20 output, 60 total tokens (reported turns: 2; unreported turns: 0)\n0 reasoning tokens\nReasoning reported for 1 of 2 turns\n12 tokens read from cache\nCache reads reported for 1 of 2 turns\n0 tokens written to cache\nCache writes reported for 1 of 2 turns\n0.5 USD reported cost\n";
        let first = report.samples.first().unwrap().seq;
        let vessel_proto::usage::UsageSampleKind::Cost { since, .. } = report.samples.first().unwrap().kind else { panic!("first fixture sample is a cost increase"); };
        let since = since.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M %Z");
        let first_turn = report.samples[1].seq;
        let last = report.samples.last().unwrap().seq;
        let session = fixture.vessel(&["usage", "session", &id.to_string()], b"").await;
        assert!(session.ok, "{}", session.stderr);
        assert_eq!(session.stdout, format!("Retained session usage\n{counts}0.5 USD latest cumulative session cost\n{accounting}claude-code / default\n{counts}Report {first}: +0.5 USD since {since}\nReport {first_turn}: 20 input, 10 output, 30 total tokens\n  0 reasoning tokens\n  12 tokens read from cache\n  0 tokens written to cache\nReport {last}: 20 input, 10 output, 30 total tokens\n  reasoning tokens not reported\n  cache reads not reported\n  cache writes not reported\n"));
        let machine = fixture.vessel(&["usage", "machine", "--period", "day"], b"").await;
        assert!(machine.ok, "{}", machine.stderr);
        assert_eq!(machine.stdout, format!("Past 24 hours\n{counts}{accounting}claude-code / default\n{counts}"));
        let agent = fixture.vessel(&["usage", "agent", "claude", "--period", "week"], b"").await;
        assert!(agent.ok, "{}", agent.stderr);
        assert_eq!(agent.stdout, format!("Past 7 days\n{counts}{accounting}claude-code / default\n{counts}"));
    }).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn saved_limits_commands_toggle_and_refresh_only_the_offline_node() {
    use vessel_harness::LimitProbe;
    within(async {
        let fixture=Fixture::start_with(|config,data| {
            let home=data.join("user-home/.codex");std::fs::create_dir_all(&home).unwrap();
            std::fs::write(home.join("auth.json"),br#"{"tokens":{"access_token":"synthetic-cli-offline-access-token"}}"#).unwrap();
            let body=data.join("limits-response.json");
            std::fs::write(&body,br#"{"rate_limit":{"primary_window":{"used_percent":0}}}"#).unwrap();
            config.limits=LimitProbe::Recorded {root:data.to_owned(),body,status:200,retry_after:None};
        }).await;
        let off=fixture.vessel(&["usage","limits","codex","--refresh"],b"").await;
        assert!(off.ok,"{}",off.stderr);
        assert_eq!(off.stdout,"codex / codex · saved sign-in limits\nExternal limit lookup is off. The usage commands still show what the agent reported.\nAn administrator can run `vessel usage lookup on` to allow this machine to read configured agents' saved access tokens (and Codex account identifiers, when present) and send them only to Anthropic or ChatGPT to read account limits.\n");
        let enabled=fixture.vessel(&["usage","lookup","on"],b"").await;assert!(enabled.ok,"{}",enabled.stderr);
        assert_eq!(enabled.stdout,"External limit lookup is enabled on this machine. A requested refresh reads the configured agent's saved access token (and Codex account identifier, when present) here and sends it only to Anthropic or ChatGPT to read account limits. Sign-in refresh stays with the agent.\n");
        let before=fixture.vessel(&["usage","limits","codex"],b"").await;assert!(before.ok,"{}",before.stderr);
        assert_eq!(before.stdout,"codex / codex · saved sign-in limits\nNo saved sign-in limits have been checked. Request a refresh to check them.\n");
        let fresh=fixture.vessel(&["usage","limits","codex","--refresh"],b"").await;assert!(fresh.ok,"{}",fresh.stderr);
        assert_eq!(fresh.stdout,"codex / codex · saved sign-in limits\nChecked less than 1 min ago.\nAccount · Primary limit, period not reported: 0% used · reset not reported\nThese are limits of the observed saved sign-in; a running agent may remember another login.\n");
        let disabled=fixture.vessel(&["usage","lookup","off"],b"").await;assert!(disabled.ok,"{}",disabled.stderr);
        assert_eq!(disabled.stdout,"External limit lookup is off. Cached external observations have been cleared.\n");
        fixture.stop().await;

    }).await;
}
