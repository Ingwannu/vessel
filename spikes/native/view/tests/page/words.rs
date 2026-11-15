//! The page's `data/timeline-text.test.ts`, `data/approval.test.ts` and
//! `data/context-meter.test.ts`: the sentences, in the page's English,
//! and the same entries in Korean.

use serde_json::json;
use vessel_app_view::timeline::{
    Entry, EntryKind, Fullness, PlanActivity, Settled, answers_of, meter_of, settled,
};
use vessel_app_view::{meter_tokens, short_tokens};
use vessel_proto::{ContextUsage, PermissionOption, PermissionOutcome, TurnUsage};

use crate::support::make::{message, tool};
use crate::support::{EN, KO};

fn options() -> Vec<PermissionOption> {
    serde_json::from_value(json!([
        { "id": "always", "name": "Always allow", "kind": "allow_always" },
        { "id": "allow", "name": "Allow", "kind": "allow_once" },
        { "id": "reject", "name": "Reject", "kind": "reject_once" },
        { "id": "never", "name": "Never", "kind": "reject_always" }
    ]))
    .unwrap()
}

fn outcome(option: &str) -> PermissionOutcome {
    serde_json::from_value(json!({ "outcome": "selected", "option": option })).unwrap()
}

fn with_kind(kind: EntryKind) -> Entry {
    Entry {
        kind,
        ..message(1, "")
    }
}

fn usage(total: u64, input: u64, output: u64) -> TurnUsage {
    serde_json::from_value(json!({ "total": total, "input": input, "output": output })).unwrap()
}

#[test]
fn an_entry_as_a_line_says_what_the_row_shows_without_its_markup() {
    assert_eq!(
        EN.spoken(&message(1, "**Done**"), catalog("en")),
        "**Done**"
    );
    let mut running = tool(2, vessel_proto::ToolKind::Read);
    if let EntryKind::Tool(view) = &mut running.kind {
        view.title = "Read status.rs".to_owned();
        view.status = vessel_proto::ToolCallStatus::InProgress;
    }
    assert_eq!(
        EN.spoken(&running, catalog("en")),
        "Read status.rs · in progress"
    );
    let ended = |usage| {
        with_kind(EntryKind::TurnEnded {
            reason: vessel_proto::StopReason::EndTurn,
            usage,
            point: false,
        })
    };
    assert_eq!(
        EN.spoken(&ended(None), catalog("en")),
        "Turn ended · end turn"
    );
    assert_eq!(
        EN.spoken(&ended(Some(usage(30, 20, 10))), catalog("en")),
        "Turn ended · end turn · 30 tokens · 20 in · 10 out"
    );
    let plan = with_kind(EntryKind::Plan {
        entries: serde_json::from_value(json!([
            { "content": "a", "priority": "high", "status": "completed" },
            { "content": "b", "priority": "low", "status": "pending" }
        ]))
        .unwrap(),
        activity: PlanActivity::Created { count: 2 },
    });
    assert_eq!(EN.spoken(&plan, catalog("en")), "Plan · 1 of 2 done");
    // A plan row's words, Paseo's.
    assert_eq!(
        EN.plan_activity(&PlanActivity::Created { count: 4 }),
        "Created 4 tasks"
    );
    assert_eq!(
        EN.plan_activity(&PlanActivity::Created { count: 1 }),
        "Created 1 task"
    );
    assert_eq!(
        KO.plan_activity(&PlanActivity::Created { count: 4 }),
        "작업 4개 생성"
    );
    assert_eq!(
        KO.plan_activity(&PlanActivity::Completed("a".to_owned())),
        "완료됨"
    );
    // The same entries in Korean.
    assert_eq!(
        KO.spoken(&running, catalog("ko")),
        "Read status.rs · 진행 중"
    );
    assert_eq!(KO.spoken(&plan, catalog("ko")), "계획 · 2개 중 1개 완료");
    assert_eq!(
        KO.spoken(&ended(Some(usage(1234, 1000, 234))), catalog("ko")),
        "턴 종료 · 정상 종료 · 토큰 1,234 · 입력 1,000 · 출력 234"
    );
}

#[test]
fn puts_the_once_options_on_the_buttons_and_the_rest_under_more() {
    let options = options();
    let answers = answers_of(&options);
    assert_eq!(answers.approve.unwrap().name, "Allow");
    assert_eq!(answers.decline.unwrap().name, "Reject");
    let more: Vec<&str> = answers
        .more
        .iter()
        .map(|option| option.name.as_str())
        .collect();
    assert_eq!(more, ["Always allow", "Never"]);
}

#[test]
fn falls_back_to_any_allow_or_reject_and_to_none() {
    let options: Vec<PermissionOption> = options()
        .into_iter()
        .filter(|option| option.id.as_str() == "always" || option.id.as_str() == "never")
        .collect();
    let answers = answers_of(&options);
    assert_eq!(answers.approve.unwrap().name, "Always allow");
    assert_eq!(answers.decline.unwrap().name, "Never");
    assert!(answers.more.is_empty());
    let none = answers_of(&[]);
    assert!(none.approve.is_none() && none.decline.is_none() && none.more.is_empty());
}

#[test]
fn says_how_a_request_ended_in_the_agents_words() {
    let options = options();
    assert_eq!(settled(&options, None), Settled::Pending);
    assert_eq!(
        settled(&options, Some(&outcome("allow"))),
        Settled::Allowed("Allow".to_owned())
    );
    assert_eq!(
        settled(&options, Some(&outcome("never"))),
        Settled::Declined("Never".to_owned())
    );
    assert_eq!(
        settled(&options, Some(&PermissionOutcome::Cancelled)),
        Settled::Cancelled
    );
    assert_eq!(
        EN.settled_line("Run fake tool", &settled(&options, Some(&outcome("allow")))),
        "Allowed · Run fake tool · Allow"
    );
    assert_eq!(
        EN.settled_line("Run fake tool", &Settled::Pending),
        "Waiting on permission · Run fake tool"
    );
}

fn context(used: u64, size: u64) -> ContextUsage {
    serde_json::from_value(json!({ "used": used, "size": size })).unwrap()
}

#[test]
fn no_meter_before_the_agent_says_or_without_a_window() {
    assert!(meter_of(None).is_none());
    assert!(meter_of(Some(&context(10, 0))).is_none());
}

#[test]
fn the_ring_its_colour_and_its_words_by_the_references_thresholds() {
    let room = meter_of(Some(&context(1200, 200_000))).unwrap();
    assert_eq!(
        (room.percent, room.fullness, room.arc),
        (1, Fullness::Room, 80)
    );
    assert_eq!(
        EN.meter_lines(&room, None),
        ["Context window", "1k / 200k tokens (1%)"]
    );
    assert_eq!(
        meter_of(Some(&context(140_000, 200_000))).unwrap().fullness,
        Fullness::Low
    );
    let full = meter_of(Some(&context(190_000, 200_000))).unwrap();
    assert_eq!(full.fullness, Fullness::Full);
    assert!(
        EN.meter_lines(&full, Some((0.5, "USD")))
            .contains(&"Session cost $0.50".to_owned())
    );
}

#[test]
fn counts_and_costs_as_written() {
    assert_eq!(meter_tokens(950), "950");
    assert_eq!(meter_tokens(84_400), "84k");
    assert_eq!(meter_tokens(1_200_000), "1.2M");
    let full = meter_of(Some(&context(190_000, 200_000))).unwrap();
    assert!(
        EN.meter_lines(&full, Some((12.5, "credits")))
            .contains(&"Session cost 12.5 credits".to_owned())
    );
    assert_eq!(
        EN.turn_tokens(&usage(30, 20, 10)),
        "30 tokens · 20 in · 10 out"
    );
}

#[test]
fn model_calls_as_vessel_log_writes_them() {
    assert_eq!(short_tokens(950), "950");
    assert_eq!(short_tokens(1200), "1.2k");
    assert_eq!(short_tokens(48_000), "48k");
    assert_eq!(short_tokens(1_000), "1k");
    assert_eq!(short_tokens(1_300_000), "1.3M");
    assert_eq!(EN.duration(40), "40 ms");
    assert_eq!(EN.duration(1234), "1.2 s");
    assert_eq!(EN.duration(125_000), "2 m 5 s");
    let call: vessel_proto::ModelCall = serde_json::from_value(json!({
        "route": "qwen-coder", "model": "qwen/qwen3-coder", "api": "openai_chat_completions",
        "usage": { "input": 72, "cache_read": 128, "cache_write": 0, "output": 50, "reasoning": 10 },
        "stop": "tool_use", "duration_ms": 1234
    }))
    .unwrap();
    assert_eq!(
        EN.model_line(&call),
        "Model call · qwen-coder · qwen/qwen3-coder · 200 in (128 cached) · 50 out · tool use · 1.2 s"
    );
}

#[test]
fn a_time_today_and_a_date_with_the_time_otherwise() {
    use chrono::{DateTime, FixedOffset, TimeZone, Utc};
    let seoul = FixedOffset::east_opt(9 * 3600).unwrap();
    let now: DateTime<FixedOffset> = seoul.with_ymd_and_hms(2026, 11, 13, 20, 0, 0).unwrap();
    let today = Utc.with_ymd_and_hms(2026, 11, 13, 5, 7, 0).unwrap();
    let earlier = Utc.with_ymd_and_hms(2026, 11, 7, 12, 0, 0).unwrap();
    assert_eq!(EN.when(today, now), "today 14:07");
    assert_eq!(KO.when(today, now), "오늘 14:07");
    assert_eq!(EN.when(earlier, now), "2026-11-07 21:00");
}

#[test]
fn a_diff_as_the_card_shows_it() {
    use vessel_app_view::timeline::{Side, diff_lines};
    let (lines, cut) = diff_lines(Some("a\n\nb"), "a\nc");
    let shown: Vec<(Side, usize, &str)> = lines
        .iter()
        .map(|line| (line.side, line.number, line.text))
        .collect();
    assert_eq!(
        shown,
        [
            (Side::Removed, 1, "a"),
            (Side::Removed, 3, "b"),
            (Side::Added, 1, "a"),
            (Side::Added, 2, "c"),
        ]
    );
    assert!(!cut);
    let long = "x\n".repeat(3000);
    let (lines, cut) = diff_lines(None, &long);
    assert_eq!(lines.len(), vessel_app_view::timeline::MAX_LINES);
    assert!(cut);
}

#[test]
fn json_as_the_card_shows_it() {
    use vessel_app_view::timeline::json_text;
    let raw = |text: &str| vessel_proto::RawJson::from_text(text).unwrap();
    assert_eq!(json_text(&raw(r#""plain""#)), "plain");
    assert_eq!(
        json_text(&raw(r#"{"path":"a.rs"}"#)),
        "{\n  \"path\": \"a.rs\"\n}"
    );
}

fn catalog(tag: &str) -> &'static vessel_app_view::localization::Translator {
    use vessel_app_view::localization::{Locale, Translator};
    Translator::get(Locale::from_tag(tag).unwrap()).unwrap()
}
