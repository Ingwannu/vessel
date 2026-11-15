//! The merge of a new answer into a shown list, at both anchors: what is
//! under the pointer never moves, the choice in effect is the agent's
//! newest, nothing is invented.

use super::*;

#[test]
fn the_same_answer_changes_nothing() {
    let shown = vec![models("b", &["a", "b", "c"]), toggle("fast", false)];
    assert_eq!(arrange(shown.clone(), &shown, Anchored::Top), shown);
}

#[test]
fn values_that_stay_keep_their_shown_places_when_the_agent_reorders_them() {
    let shown = vec![models("a", &["a", "b", "c", "d"])];
    let fresh = vec![models("a", &["d", "c", "b", "a"])];
    let arranged = arrange(fresh, &shown, Anchored::Top);
    assert_eq!(value_ids(&arranged, "model"), ["a", "b", "c", "d"]);
}

#[test]
fn a_new_value_goes_after_the_shown_ones_wherever_the_agent_puts_it() {
    let shown = vec![models("a", &["a", "b", "c"])];
    let fresh = vec![models("a", &["new", "a", "b", "c"])];
    let arranged = arrange(fresh, &shown, Anchored::Top);
    assert_eq!(value_ids(&arranged, "model"), ["a", "b", "c", "new"]);
}

#[test]
fn in_a_list_anchored_at_its_bottom_new_values_go_at_the_top_in_the_agents_order() {
    let shown = vec![models("a", &["a", "b", "c"])];
    let fresh = vec![models("a", &["a", "new1", "b", "c", "new2"])];
    let arranged = arrange(fresh, &shown, Anchored::Bottom);
    assert_eq!(
        value_ids(&arranged, "model"),
        ["new1", "new2", "a", "b", "c"]
    );
}

#[test]
fn in_a_list_anchored_at_its_bottom_new_options_go_first() {
    let shown = vec![models("a", &["a"]), toggle("fast", false)];
    let fresh = vec![
        models("a", &["a"]),
        toggle("web", true),
        toggle("fast", false),
    ];
    let arranged = arrange(fresh, &shown, Anchored::Bottom);
    assert_eq!(option_ids(&arranged), ["web", "model", "fast"]);
}

#[test]
fn a_value_gone_from_the_answer_leaves_the_list() {
    let shown = vec![models("a", &["a", "b", "c"])];
    let fresh = vec![models("a", &["a", "c"])];
    let arranged = arrange(fresh, &shown, Anchored::Top);
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
    let arranged = arrange(fresh, &shown, Anchored::Top);
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
    let arranged = arrange(vec![fresh], &shown, Anchored::Top);
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
    let arranged = arrange(fresh, &shown, Anchored::Top);
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
fn at_the_bottom_anchor_a_new_value_joins_its_group_and_a_new_group_keeps_its_order() {
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
            grouped("g2", "Google"),
        ],
    )];
    let arranged = arrange(fresh, &shown, Anchored::Bottom);
    assert_eq!(
        value_ids(&arranged, "model"),
        ["g1", "g2", "a3", "a1", "a2", "o2", "o1"]
    );
}

#[test]
fn an_option_whose_kind_changed_is_taken_as_answered() {
    let shown = vec![models("a", &["a", "b"])];
    let fresh = vec![SessionOption {
        kind: OptionKind::Toggle { on: true },
        ..models("a", &[])
    }];
    let arranged = arrange(fresh.clone(), &shown, Anchored::Top);
    assert_eq!(arranged, fresh);
}

#[test]
fn nothing_shown_takes_the_answer_as_sent_and_an_empty_answer_empties() {
    let fresh = vec![models("c", &["c", "a", "b"]), toggle("fast", true)];
    assert_eq!(arrange(fresh.clone(), &[], Anchored::Top), fresh);
    assert_eq!(arrange(fresh.clone(), &[], Anchored::Bottom), fresh);
    assert!(arrange(Vec::new(), &fresh, Anchored::Top).is_empty());
}

#[test]
fn a_duplicate_id_in_an_answer_is_neither_lost_nor_invented() {
    let shown = vec![models("a", &["a", "b"])];
    let fresh = vec![models("a", &["a", "b", "a"])];
    let arranged = arrange(fresh, &shown, Anchored::Top);
    assert_eq!(value_ids(&arranged, "model"), ["a", "b", "a"]);
}

/// Every answer made of some of the shown values, in any order, with a
/// new one anywhere: the values that stay are in the shown order, the new
/// one at the edge away from the anchor, and the rows between that anchor
/// and the nearest one gone never move on screen (at the top anchor their
/// indexes stay; at the bottom anchor their distance from the end does).
#[test]
fn for_every_reordering_the_rows_that_stay_keep_their_order_and_places() {
    let shown_ids = ["a", "b", "c", "d"];
    let shown = vec![models("a", &shown_ids)];
    let mut cases = Vec::new();
    // Each shown value kept or not (16 subsets), each subset in every
    // order, a new value at each place, both anchors.
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
                let kept_ids: Vec<String> = kept.iter().map(|id| (*id).to_owned()).collect();
                // The top anchor: the new one last, the prefix unmoved.
                let top = value_ids(
                    &arrange(vec![models("a", &fresh_ids)], &shown, Anchored::Top),
                    "model",
                );
                let mut expected = kept_ids.clone();
                expected.push("new".to_owned());
                assert_eq!(top, expected, "top, fresh {fresh_ids:?}");
                let first_gone = shown_ids
                    .iter()
                    .position(|id| !kept.contains(id))
                    .unwrap_or(shown_ids.len());
                assert_eq!(&top[..first_gone], &shown_ids[..first_gone]);
                // The bottom anchor: the new one first, the suffix unmoved.
                let bottom = value_ids(
                    &arrange(vec![models("a", &fresh_ids)], &shown, Anchored::Bottom),
                    "model",
                );
                let mut expected = vec!["new".to_owned()];
                expected.extend(kept_ids);
                assert_eq!(bottom, expected, "bottom, fresh {fresh_ids:?}");
                let after_last_gone = shown_ids
                    .iter()
                    .rposition(|id| !kept.contains(id))
                    .map_or(0, |last| last.saturating_add(1));
                let tail = shown_ids.len().saturating_sub(after_last_gone);
                assert_eq!(
                    &bottom[bottom.len().saturating_sub(tail)..],
                    &shown_ids[after_last_gone..]
                );
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
