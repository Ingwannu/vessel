//! The store: what a control shows, when to ask again, what was kept,
//! and the agents' commands beside their options.

use super::*;

#[test]
fn the_first_time_is_loading_then_the_list() {
    let mut offers = Offers::default();
    let now = Instant::now();
    assert_eq!(offers.offered(&key()), Offered::Loading);
    assert!(offers.want(&key(), Why::Open, now));
    assert_eq!(offers.offered(&key()), Offered::Loading);
    let list = vec![models("a", &["a", "b"])];
    assert_eq!(
        offers.answered(key(), Ok(list.clone()), None, now),
        Arrival::Changed
    );
    assert_eq!(
        offers.offered(&key()),
        Offered::Ready {
            options: &list,
            freshness: Freshness::Fresh
        }
    );
}

#[test]
fn a_kept_list_is_shown_at_once_and_refreshed_in_place() {
    let mut offers = Offers::default();
    let now = Instant::now();
    let kept = vec![models("a", &["a", "b"])];
    assert_eq!(
        offers.kept(key(), Some(kept.clone()), None),
        Arrival::Changed
    );
    assert_eq!(
        offers.offered(&key()),
        Offered::Ready {
            options: &kept,
            freshness: Freshness::Kept
        },
        "no spinner when a copy exists"
    );
    // The refresh goes out at once (kept, never refreshed this run).
    assert!(offers.want(&key(), Why::Open, now));
    // The list stays while it is in flight.
    assert!(matches!(offers.offered(&key()), Offered::Ready { .. }));
    let fresh = vec![models("b", &["c", "a", "b"])];
    assert_eq!(
        offers.answered(key(), Ok(fresh), Some(Anchored::Bottom), now),
        Arrival::Changed
    );
    let options = offers.options(&key()).unwrap_or_default();
    // Open above its control: the new value at the top, the rest in place.
    assert_eq!(
        value_ids(options, "model"),
        ["c", "a", "b"],
        "open: laid over"
    );
    assert_eq!(current_of(options, "model").as_deref(), Some("b"));
}

#[test]
fn a_closed_list_takes_the_agents_order() {
    let mut offers = Offers::default();
    let now = Instant::now();
    let _kept = offers.kept(key(), Some(vec![models("a", &["a", "b"])]), None);
    let fresh = vec![models("a", &["b", "a"])];
    assert_eq!(
        offers.answered(key(), Ok(fresh.clone()), None, now),
        Arrival::Changed
    );
    assert_eq!(offers.options(&key()), Some(fresh.as_slice()));
}

#[test]
fn the_same_answer_is_same_and_confirms_the_list() {
    let mut offers = Offers::default();
    let now = Instant::now();
    let list = vec![models("a", &["a", "b"])];
    let _kept = offers.kept(key(), Some(list.clone()), None);
    assert_eq!(
        offers.answered(key(), Ok(list.clone()), None, now),
        Arrival::Same
    );
    assert_eq!(
        offers.offered(&key()),
        Offered::Ready {
            options: &list,
            freshness: Freshness::Fresh
        }
    );
}

#[test]
fn a_failed_refresh_keeps_the_list_marked_stale_until_one_succeeds() {
    let mut offers = Offers::default();
    let now = Instant::now();
    let list = vec![models("a", &["a", "b"])];
    let _kept = offers.kept(key(), Some(list.clone()), None);
    assert_eq!(
        offers.answered(key(), Err("unreachable".to_owned()), None, now),
        Arrival::KeptStale
    );
    assert_eq!(
        offers.offered(&key()),
        Offered::Ready {
            options: &list,
            freshness: Freshness::Stale
        }
    );
    let later = after(now, RETRY_GAP);
    assert!(offers.want(&key(), Why::Open, later));
    assert_eq!(
        offers.answered(key(), Ok(list.clone()), None, later),
        Arrival::Same
    );
    assert_eq!(
        offers.offered(&key()),
        Offered::Ready {
            options: &list,
            freshness: Freshness::Fresh
        }
    );
}

#[test]
fn a_first_failure_says_why_and_a_retry_is_loading_again() {
    let mut offers = Offers::default();
    let now = Instant::now();
    assert!(offers.want(&key(), Why::Open, now));
    assert_eq!(
        offers.answered(key(), Err("agent missing".to_owned()), None, now),
        Arrival::Failed
    );
    assert_eq!(offers.offered(&key()), Offered::Failed("agent missing"));
    assert!(offers.want(&key(), Why::Open, after(now, RETRY_GAP)));
    assert_eq!(offers.offered(&key()), Offered::Loading);
}

#[test]
fn a_kept_list_never_replaces_an_answer_of_this_run() {
    let mut offers = Offers::default();
    let now = Instant::now();
    let fresh = vec![models("b", &["b"])];
    let _first = offers.answered(key(), Ok(fresh.clone()), None, now);
    assert_eq!(
        offers.kept(key(), Some(vec![models("a", &["a"])]), None),
        Arrival::Same
    );
    assert_eq!(offers.options(&key()), Some(fresh.as_slice()));
}

#[test]
fn ten_opens_are_one_request() {
    let mut offers = Offers::default();
    let now = Instant::now();
    let asked = [0_u64, 50, 100, 150, 200, 250, 300, 350, 400, 450]
        .into_iter()
        .filter(|ms| offers.want(&key(), Why::Open, after(now, Duration::from_millis(*ms))))
        .count();
    assert_eq!(asked, 1);
    // Answered: still none within the gap, one after it.
    let _answer = offers.answered(key(), Ok(vec![models("a", &["a"])]), None, now);
    let opens = (0..10_u64)
        .filter(|step| offers.want(&key(), Why::Open, after(now, secs(*step))))
        .count();
    assert_eq!(opens, 0);
    assert!(offers.want(&key(), Why::Open, after(now, OPEN_GAP)));
}

#[test]
fn a_failure_is_retried_on_open_only_after_its_gap() {
    let mut offers = Offers::default();
    let now = Instant::now();
    let _failed = offers.answered(key(), Err("offline".to_owned()), None, now);
    assert!(!offers.should_ask(&key(), Why::Open, after(now, secs(1))));
    assert!(offers.should_ask(&key(), Why::Open, after(now, RETRY_GAP)));
}

#[test]
fn a_connection_or_a_change_of_agents_asks_within_the_gap_but_not_twice() {
    let mut offers = Offers::default();
    let now = Instant::now();
    let _answer = offers.answered(key(), Ok(vec![models("a", &["a"])]), None, now);
    let soon = after(now, secs(1));
    assert!(!offers.should_ask(&key(), Why::Open, soon));
    assert!(offers.want(&key(), Why::Connect, soon));
    assert!(!offers.want(&key(), Why::AgentsChanged, soon), "in flight");
}

#[test]
fn a_persons_retry_asks_at_once_unless_one_is_in_flight() {
    let mut offers = Offers::default();
    let now = Instant::now();
    let _answer = offers.answered(key(), Ok(vec![models("a", &["a"])]), None, now);
    assert!(!offers.asking(&key(), now));
    assert!(offers.want(&key(), Why::Retry, after(now, secs(1))));
    assert!(offers.asking(&key(), after(now, secs(1))));
    assert!(!offers.want(&key(), Why::Retry, after(now, secs(2))));
}

#[test]
fn a_request_never_answered_stops_holding_back_after_its_timeout() {
    let mut offers = Offers::default();
    let now = Instant::now();
    assert!(offers.want(&key(), Why::Open, now));
    assert!(!offers.want(&key(), Why::Open, after(now, secs(59))));
    assert!(offers.want(&key(), Why::Open, after(now, ASK_TIMEOUT)));
}

#[test]
fn lists_follow_a_renamed_machine_and_leave_with_a_forgotten_one() {
    let mut offers = Offers::default();
    let list = vec![models("a", &["a"])];
    let _kept = offers.kept(key(), Some(list.clone()), None);
    let codex = OfferKey::new("laptop", HarnessRef::Known(HarnessKind::Codex));
    let _other = offers.kept(codex, Some(list.clone()), None);
    let elsewhere = OfferKey::new("desk", HarnessRef::Known(HarnessKind::Codex));
    let _desk = offers.kept(elsewhere.clone(), Some(list.clone()), None);
    assert_eq!(offers.agents_on("laptop").len(), 2);
    offers.renamed("laptop", "studio");
    assert_eq!(offers.options(&key()), None);
    let moved = OfferKey::new("studio", HarnessRef::Known(HarnessKind::ClaudeCode));
    assert_eq!(offers.options(&moved), Some(list.as_slice()));
    offers.forgotten("studio");
    assert!(offers.agents_on("studio").is_empty());
    assert_eq!(offers.options(&elsewhere), Some(list.as_slice()));
}

#[test]
fn two_machines_never_see_each_others_lists() {
    let mut offers = Offers::default();
    let now = Instant::now();
    let laptop = key();
    let desk = OfferKey::new("desk", HarnessRef::Known(HarnessKind::ClaudeCode));
    let _answer = offers.answered(laptop, Ok(vec![models("a", &["a"])]), None, now);
    assert_eq!(offers.offered(&desk), Offered::Loading);
    assert!(offers.should_ask(&desk, Why::Open, now));
}

/// Paseo's `resolveEffectiveComposerModeId`: a remembered choice the agent
/// no longer offers falls back to what is shown, and what is sent matches
/// it. Here what is shown is the agent's own value, and nothing is sent
/// for the vanished pick.
#[test]
fn a_kept_pick_the_agent_stopped_offering_falls_back_to_the_agents_value() {
    let mut offers = Offers::default();
    let now = Instant::now();
    let _kept = offers.kept(key(), Some(vec![models("a", &["a", "b"])]), None);
    let mut picks = crate::Picks::default();
    picks.pick(vessel_proto::OptionChoice {
        id: OptionId::new("model"),
        value: vessel_proto::ChoiceValue::Select {
            value: OptionValueId::new("b"),
        },
    });
    let shown = crate::with_picks(offers.options(&key()).unwrap_or_default(), &picks);
    assert_eq!(current_of(&shown, "model").as_deref(), Some("b"));
    // The agent drops "b" while the pick is kept.
    let _answer = offers.answered(
        key(),
        Ok(vec![models("a", &["a", "c"])]),
        Some(Anchored::Bottom),
        now,
    );
    let options = offers.options(&key()).unwrap_or_default();
    let shown = crate::with_picks(options, &picks);
    assert_eq!(current_of(&shown, "model").as_deref(), Some("a"));
    assert!(
        crate::choices_for(options, &picks).is_empty(),
        "nothing sent for it"
    );
}

fn command(name: &str) -> AgentCommand {
    AgentCommand {
        name: name.parse().unwrap(),
        description: format!("{name}, as the agent says"),
        hint: None,
    }
}

#[test]
fn an_agents_commands_are_kept_beside_its_options_and_reported_ones_win() {
    let mut offers = Offers::default();
    assert_eq!(offers.commands(&key()), None, "none known yet");
    // Kept from an earlier run: shown before any session reports.
    let kept = vec![command("review"), command("compact")];
    assert_eq!(
        offers.kept(key(), None, Some(kept.clone())),
        Arrival::Changed
    );
    assert_eq!(offers.commands(&key()), Some(kept.as_slice()));
    assert!(
        offers.options(&key()).is_none(),
        "commands alone are no list"
    );
    assert_eq!(offers.offered(&key()), Offered::Loading);
    // A session reports the same: nothing to keep again.
    assert!(!offers.commands_reported(key(), &kept));
    // It reports others: they replace the kept ones and are to be kept.
    let reported = vec![command("review"), command("init")];
    assert!(offers.commands_reported(key(), &reported));
    assert_eq!(offers.commands(&key()), Some(reported.as_slice()));
    // A later hand-over does not undo what this run heard.
    assert_eq!(offers.kept(key(), None, Some(kept)), Arrival::Same);
    assert_eq!(offers.commands(&key()), Some(reported.as_slice()));
    // An agent with commands counts as one the machine's connection asks.
    assert_eq!(offers.agents_on("laptop").len(), 1);
}

#[test]
fn an_agent_that_offers_no_commands_is_known_to_offer_none() {
    let mut offers = Offers::default();
    assert!(offers.commands_reported(key(), &[]));
    assert_eq!(offers.commands(&key()), Some(&[][..]));
    assert!(!offers.commands_reported(key(), &[]));
}
