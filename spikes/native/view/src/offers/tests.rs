//! The store's rules, the merge of a new answer into a shown list hardest:
//! what is under the pointer never moves, the choice in effect is the
//! agent's newest, nothing is invented.

use std::time::{Duration, Instant};

use vessel_proto::{
    HarnessKind, HarnessRef, OptionCategory, OptionId, OptionKind, OptionValue, OptionValueId,
    SessionOption,
};

use super::*;

fn value(id: &str) -> OptionValue {
    OptionValue {
        id: OptionValueId::new(id),
        name: id.to_uppercase(),
        description: None,
        group: None,
    }
}

fn grouped(id: &str, group: &str) -> OptionValue {
    OptionValue {
        group: Some(group.to_owned()),
        ..value(id)
    }
}

fn select(id: &str, current: &str, values: Vec<OptionValue>) -> SessionOption {
    SessionOption {
        id: OptionId::new(id),
        name: id.to_owned(),
        description: None,
        category: OptionCategory::Model,
        kind: OptionKind::Select {
            current: OptionValueId::new(current),
            values,
        },
    }
}

fn toggle(id: &str, on: bool) -> SessionOption {
    SessionOption {
        id: OptionId::new(id),
        name: id.to_owned(),
        description: None,
        category: OptionCategory::Other,
        kind: OptionKind::Toggle { on },
    }
}

fn models(current: &str, ids: &[&str]) -> SessionOption {
    select("model", current, ids.iter().map(|id| value(id)).collect())
}

/// The value ids of option `id`, in order.
fn value_ids(options: &[SessionOption], id: &str) -> Vec<String> {
    options
        .iter()
        .find(|option| option.id.as_str() == id)
        .map(|option| match &option.kind {
            OptionKind::Select { values, .. } => values
                .iter()
                .map(|value| value.id.as_str().to_owned())
                .collect(),
            OptionKind::Toggle { .. } | OptionKind::Unknown => Vec::new(),
        })
        .unwrap_or_default()
}

fn option_ids(options: &[SessionOption]) -> Vec<&str> {
    options.iter().map(|option| option.id.as_str()).collect()
}

fn current_of(options: &[SessionOption], id: &str) -> Option<String> {
    options
        .iter()
        .find(|option| option.id.as_str() == id)
        .and_then(|option| match &option.kind {
            OptionKind::Select { current, .. } => Some(current.as_str().to_owned()),
            OptionKind::Toggle { .. } | OptionKind::Unknown => None,
        })
}

/// `now`, `by` later.
fn after(now: Instant, by: Duration) -> Instant {
    now.checked_add(by).unwrap()
}

fn secs(count: u64) -> Duration {
    Duration::from_secs(count)
}

fn key() -> OfferKey {
    OfferKey::new("laptop", HarnessRef::Known(HarnessKind::ClaudeCode))
}

// The merge.

#[test]
fn the_same_answer_changes_nothing() {
    let shown = vec![models("b", &["a", "b", "c"]), toggle("fast", false)];
    assert_eq!(arrange(shown.clone(), &shown), shown);
}

#[test]
fn values_that_stay_keep_their_shown_places_when_the_agent_reorders_them() {
    let shown = vec![models("a", &["a", "b", "c", "d"])];
    let fresh = vec![models("a", &["d", "c", "b", "a"])];
    let arranged = arrange(fresh, &shown);
    assert_eq!(value_ids(&arranged, "model"), ["a", "b", "c", "d"]);
}

#[test]
fn a_new_value_goes_after_the_shown_ones_wherever_the_agent_puts_it() {
    let shown = vec![models("a", &["a", "b", "c"])];
    let fresh = vec![models("a", &["new", "a", "b", "c"])];
    let arranged = arrange(fresh, &shown);
    assert_eq!(value_ids(&arranged, "model"), ["a", "b", "c", "new"]);
}

#[test]
fn a_value_gone_from_the_answer_leaves_the_list() {
    let shown = vec![models("a", &["a", "b", "c"])];
    let fresh = vec![models("a", &["a", "c"])];
    let arranged = arrange(fresh, &shown);
    assert_eq!(value_ids(&arranged, "model"), ["a", "c"]);
}

#[test]
fn new_options_follow_the_shown_ones_and_gone_ones_leave() {
    let shown = vec![
        models("a", &["a"]),
        toggle("fast", false),
        select("mode", "plan", vec![value("plan")]),
    ];
    let fresh = vec![
        toggle("web", true),
        select("mode", "plan", vec![value("plan")]),
        models("a", &["a"]),
    ];
    let arranged = arrange(fresh, &shown);
    assert_eq!(option_ids(&arranged), ["model", "mode", "web"]);
}

#[test]
fn names_descriptions_and_the_value_in_effect_are_the_answers() {
    let shown = vec![models("a", &["a", "b"])];
    let mut renamed = value("b");
    renamed.name = "B, renamed".to_owned();
    renamed.description = Some("now with a description".to_owned());
    let mut fresh = select("model", "b", vec![value("a"), renamed.clone()]);
    fresh.name = "Model (new name)".to_owned();
    let arranged = arrange(vec![fresh], &shown);
    assert_eq!(current_of(&arranged, "model").as_deref(), Some("b"));
    assert_eq!(
        arranged.first().map(|option| option.name.as_str()),
        Some("Model (new name)")
    );
    let OptionKind::Select { values, .. } = &arranged[0].kind else {
        panic!("still a select");
    };
    assert_eq!(values.get(1), Some(&renamed));
}

#[test]
fn a_new_value_joins_its_group_so_a_group_is_drawn_once() {
    let shown = vec![select(
        "model",
        "a1",
        vec![
            grouped("a1", "Anthropic"),
            grouped("a2", "Anthropic"),
            grouped("o1", "OpenAI"),
        ],
    )];
    let fresh = vec![select(
        "model",
        "a1",
        vec![
            grouped("o1", "OpenAI"),
            grouped("o2", "OpenAI"),
            grouped("a3", "Anthropic"),
            grouped("a1", "Anthropic"),
            grouped("a2", "Anthropic"),
            grouped("g1", "Google"),
        ],
    )];
    let arranged = arrange(fresh, &shown);
    assert_eq!(
        value_ids(&arranged, "model"),
        ["a1", "a2", "a3", "o1", "o2", "g1"]
    );
    // Every group's values are adjacent.
    let OptionKind::Select { values, .. } = &arranged[0].kind else {
        panic!("still a select");
    };
    let groups: Vec<_> = values.iter().map(|value| value.group.clone()).collect();
    let mut runs = groups.clone();
    runs.dedup();
    let mut distinct = runs.clone();
    distinct.sort();
    distinct.dedup();
    assert_eq!(
        runs.len(),
        distinct.len(),
        "a group drawn twice: {groups:?}"
    );
}

#[test]
fn an_option_whose_kind_changed_is_taken_as_answered() {
    let shown = vec![models("a", &["a", "b"])];
    let fresh = vec![SessionOption {
        kind: OptionKind::Toggle { on: true },
        ..models("a", &[])
    }];
    let arranged = arrange(fresh.clone(), &shown);
    assert_eq!(arranged, fresh);
}

#[test]
fn nothing_shown_takes_the_answer_as_sent_and_an_empty_answer_empties() {
    let fresh = vec![models("c", &["c", "a", "b"]), toggle("fast", true)];
    assert_eq!(arrange(fresh.clone(), &[]), fresh);
    assert!(arrange(Vec::new(), &fresh).is_empty());
}

#[test]
fn a_duplicate_id_in_an_answer_is_neither_lost_nor_invented() {
    let shown = vec![models("a", &["a", "b"])];
    let fresh = vec![models("a", &["a", "b", "a"])];
    let arranged = arrange(fresh, &shown);
    assert_eq!(value_ids(&arranged, "model"), ["a", "b", "a"]);
}

/// Every answer made of some of the shown values, in any order, with new
/// ones anywhere: the values that stay are in the shown order, and the
/// rows above the first one gone never move (the pointer's row, wherever
/// it is above that, keeps its index); every fresh value is there once.
#[test]
fn for_every_reordering_the_rows_that_stay_keep_their_order_and_places() {
    let shown_ids = ["a", "b", "c", "d"];
    let shown = vec![models("a", &shown_ids)];
    let mut cases = Vec::new();
    // Each shown value kept or not (16 subsets), each subset in every
    // order, a new value at each place.
    for mask in 0_u32..16 {
        let kept: Vec<&str> = shown_ids
            .iter()
            .enumerate()
            .filter(|(index, _)| mask.checked_shr(u32::try_from(*index).unwrap()).unwrap() & 1 == 1)
            .map(|(_, id)| *id)
            .collect();
        for order in permutations(&kept) {
            for at in 0..=order.len() {
                let mut fresh_ids = order.clone();
                fresh_ids.insert(at, "new");
                let arranged = arrange(vec![models("a", &fresh_ids)], &shown);
                let got = value_ids(&arranged, "model");
                let expected: Vec<String> = kept
                    .iter()
                    .map(|id| (*id).to_owned())
                    .chain(std::iter::once("new".to_owned()))
                    .collect();
                assert_eq!(got, expected, "fresh {fresh_ids:?}");
                let first_gone = shown_ids
                    .iter()
                    .position(|id| !kept.contains(id))
                    .unwrap_or(shown_ids.len());
                assert_eq!(&got[..first_gone], &shown_ids[..first_gone]);
                cases.push(fresh_ids);
            }
        }
    }
    assert!(cases.len() > 100, "{} cases", cases.len());
}

fn permutations<'a>(items: &[&'a str]) -> Vec<Vec<&'a str>> {
    if items.len() <= 1 {
        return vec![items.to_vec()];
    }
    let mut all = Vec::new();
    for (index, first) in items.iter().enumerate() {
        let mut rest = items.to_vec();
        let _taken = rest.remove(index);
        for mut tail in permutations(&rest) {
            tail.insert(0, *first);
            all.push(tail);
        }
    }
    all
}

// The store.

#[test]
fn the_first_time_is_loading_then_the_list() {
    let mut offers = Offers::default();
    let now = Instant::now();
    assert_eq!(offers.offered(&key()), Offered::Loading);
    assert!(offers.want(&key(), Why::Open, now));
    assert_eq!(offers.offered(&key()), Offered::Loading);
    let list = vec![models("a", &["a", "b"])];
    assert_eq!(
        offers.answered(key(), Ok(list.clone()), false, now),
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
    assert_eq!(offers.kept(key(), kept.clone()), Arrival::Changed);
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
        offers.answered(key(), Ok(fresh), true, now),
        Arrival::Changed
    );
    let options = offers.options(&key()).unwrap_or_default();
    assert_eq!(
        value_ids(options, "model"),
        ["a", "b", "c"],
        "open: laid over"
    );
    assert_eq!(current_of(options, "model").as_deref(), Some("b"));
}

#[test]
fn a_closed_list_takes_the_agents_order() {
    let mut offers = Offers::default();
    let now = Instant::now();
    let _kept = offers.kept(key(), vec![models("a", &["a", "b"])]);
    let fresh = vec![models("a", &["b", "a"])];
    assert_eq!(
        offers.answered(key(), Ok(fresh.clone()), false, now),
        Arrival::Changed
    );
    assert_eq!(offers.options(&key()), Some(fresh.as_slice()));
}

#[test]
fn the_same_answer_is_same_and_confirms_the_list() {
    let mut offers = Offers::default();
    let now = Instant::now();
    let list = vec![models("a", &["a", "b"])];
    let _kept = offers.kept(key(), list.clone());
    assert_eq!(
        offers.answered(key(), Ok(list.clone()), false, now),
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
    let _kept = offers.kept(key(), list.clone());
    assert_eq!(
        offers.answered(key(), Err("unreachable".to_owned()), false, now),
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
        offers.answered(key(), Ok(list.clone()), false, later),
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
        offers.answered(key(), Err("agent missing".to_owned()), false, now),
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
    let _first = offers.answered(key(), Ok(fresh.clone()), false, now);
    assert_eq!(offers.kept(key(), vec![models("a", &["a"])]), Arrival::Same);
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
    let _answer = offers.answered(key(), Ok(vec![models("a", &["a"])]), false, now);
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
    let _failed = offers.answered(key(), Err("offline".to_owned()), false, now);
    assert!(!offers.should_ask(&key(), Why::Open, after(now, secs(1))));
    assert!(offers.should_ask(&key(), Why::Open, after(now, RETRY_GAP)));
}

#[test]
fn a_connection_or_a_change_of_agents_asks_within_the_gap_but_not_twice() {
    let mut offers = Offers::default();
    let now = Instant::now();
    let _answer = offers.answered(key(), Ok(vec![models("a", &["a"])]), false, now);
    let soon = after(now, secs(1));
    assert!(!offers.should_ask(&key(), Why::Open, soon));
    assert!(offers.want(&key(), Why::Connect, soon));
    assert!(!offers.want(&key(), Why::AgentsChanged, soon), "in flight");
}

#[test]
fn a_persons_retry_asks_at_once_unless_one_is_in_flight() {
    let mut offers = Offers::default();
    let now = Instant::now();
    let _answer = offers.answered(key(), Ok(vec![models("a", &["a"])]), false, now);
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
    let _kept = offers.kept(key(), list.clone());
    let codex = OfferKey::new("laptop", HarnessRef::Known(HarnessKind::Codex));
    let _other = offers.kept(codex, list.clone());
    let elsewhere = OfferKey::new("desk", HarnessRef::Known(HarnessKind::Codex));
    let _desk = offers.kept(elsewhere.clone(), list.clone());
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
    let _answer = offers.answered(laptop, Ok(vec![models("a", &["a"])]), false, now);
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
    let _kept = offers.kept(key(), vec![models("a", &["a", "b"])]);
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
    let _answer = offers.answered(key(), Ok(vec![models("a", &["a", "c"])]), true, now);
    let options = offers.options(&key()).unwrap_or_default();
    let shown = crate::with_picks(options, &picks);
    assert_eq!(current_of(&shown, "model").as_deref(), Some("a"));
    assert!(
        crate::choices_for(options, &picks).is_empty(),
        "nothing sent for it"
    );
}
