//! Paseo protocol `search/text-match.test.ts` and app
//! `command-center/results.test.ts`: identical text inputs and outcomes.

use super::{Field, Query, Weight};

fn one_span(range: std::ops::Range<usize>) -> Vec<std::ops::Range<usize>> {
    std::iter::once(range).collect()
}

#[test]
fn exact_word_prefix_boundary_substring_and_subsequence_tiers() {
    let cases = [
        ("PI", "pi", 0, 0, 2),
        ("feat", "feat/pi-direct-sdk", 1, 0, 4),
        ("pi", "feat/pi-direct-sdk", 1, 5, 2),
        ("pi", "a b pi c d", 1, 4, 2),
        ("par", "party", 2, 0, 3),
        ("par", "a/party", 3, 2, 3),
        ("art", "party", 4, 1, 3),
        ("trmnl", "Fix terminal resizing", 5, 4, 8),
        ("ab", "xxab x-ab-y", 1, 7, 2),
        ("pi", "pi-a pi-b", 1, 0, 2),
    ];
    for (query, text, tier, offset, spread) in cases {
        let token: Vec<_> = query.to_lowercase().chars().collect();
        assert_eq!(
            Field::new(text).hit(&token).map(|hit| hit.weight),
            Some(Weight {
                tier,
                offset,
                spread
            }),
            "{query} in {text}"
        );
    }
}

#[test]
fn separators_are_word_boundaries() {
    for separator in ["/", "-", "_", " ", ".", ":"] {
        let text = format!("x{separator}pi");
        assert_eq!(Field::new(&text).hit(&['p', 'i']).unwrap().weight.tier, 1);
    }
}

#[test]
fn subsequences_stay_inside_a_whitespace_delimited_word() {
    for separator in [" ", "\t", "\n", "\u{a0}", "\u{feff}"] {
        assert!(
            Query::new("terminal")
                .score(&[&format!("term{separator}inal")], None)
                .is_none()
        );
    }
    for query in [
        "design",
        "lab des",
        "des lab",
        "Label as Design",
        "lbl dsgn",
        "lab lab",
    ] {
        assert!(
            Query::new(query)
                .score(&["Label as Design"], None)
                .is_some(),
            "{query}"
        );
    }
    for query in ["labdes", "lasdsgn", "lab zzz"] {
        assert!(
            Query::new(query)
                .score(&["Label as Design"], None)
                .is_none(),
            "{query}"
        );
    }
}

#[test]
fn tokens_can_land_in_different_fields_and_in_any_order() {
    for query in ["changes tree", "tree changes"] {
        assert!(
            Query::new(query)
                .score(&["Open Changes", "Review the working tree"], None)
                .is_some()
        );
    }
    assert!(
        Query::new("stipe bulling")
            .score(&["add stripe billing"], None)
            .is_none()
    );
    assert!(Query::new("   ").score(&["anything"], None).is_some());
}

#[test]
fn javascript_whitespace_treats_bom_and_next_line_differently() {
    assert!(
        Query::new("terminal")
            .score(&["term\u{85}inal"], None)
            .is_some()
    );
    assert!(
        Query::new("term\u{85}inal")
            .score(&["term inal"], None)
            .is_none()
    );
    assert!(
        Query::new("term\u{feff}inal")
            .score(&["term inal"], None)
            .is_some()
    );
}

#[test]
fn visible_labels_rank_ahead_of_exact_or_prefix_hidden_keywords() {
    for query in ["a", "au", "aut", "auto", "automation"] {
        let parsed = Query::new(query);
        let visible = parsed.score(&["Automation hub"], None).unwrap();
        let hidden = parsed.score(&["Schedules"], Some("automation")).unwrap();
        assert!(visible < hidden, "{query}");
    }
}

#[test]
fn built_in_rows_ignore_position_within_the_same_match_tier() {
    let parsed = Query::new("usage");
    let earlier = parsed.score(&["Deliver sidebar usage"], None).unwrap();
    let later = parsed
        .score(&["design the cli interface to expose usage data"], None)
        .unwrap();
    assert_eq!(earlier.built_in_cmp(later), std::cmp::Ordering::Equal);
}

#[test]
fn marks_the_best_occurrence_and_every_token_or_subsequence_run() {
    for (query, text, expected) in [
        ("stripe", "add stripe billing", one_span(4..10)),
        ("bill", "unbilled usage", one_span(2..6)),
        ("main", "main", one_span(0..4)),
        ("confg", "configuration", vec![0..4, 5..6]),
        ("confg", "check configuration", vec![6..10, 11..12]),
        (
            "lbl dsgn",
            "Label as Design",
            vec![0..1, 2..3, 4..5, 9..10, 11..12, 13..15],
        ),
        ("view", "Review the view", one_span(11..15)),
        ("", "anything", vec![]),
    ] {
        assert_eq!(Query::new(query).marks(text), expected, "{query} in {text}");
    }
}

#[test]
fn unicode_highlights_address_original_bytes_while_ranking_uses_utf16_offsets() {
    assert_eq!(Query::new("이동").marks("보드로 이동"), vec![10..16]);
    assert_eq!(Query::new("été").marks("ÉTÉ été"), vec![0..5]);
    assert_eq!(
        Field::new("🦀 pi").hit(&['p', 'i']).unwrap().weight.offset,
        3
    );
    assert_eq!(Query::new("i").marks("İ"), vec![0..2]);
}

#[test]
fn typo_mode_is_opt_in_and_obeys_reference_token_budgets() {
    for (query, text) in [
        ("brnach", "switch the branch"),
        ("confug", "revamp configuration editor"),
        ("bulling", "add stripe billing"),
        ("mian", "switch to main"),
        ("configurtaion", "configuration"),
        ("strípe", "stripe"),
    ] {
        assert!(
            Query::new(query).score(&[text], None).is_none(),
            "default changed: {query}"
        );
        assert!(
            Query::typo_tolerant(query).score(&[text], None).is_some(),
            "{query}"
        );
    }
    for (query, text) in [
        ("ip", "pi"),
        ("mial", "main"),
        ("rain", "main"),
        ("maid", "main"),
        ("stpie", "add stripe billing"),
        ("zzzzzz", "add stripe billing"),
        ("terminal", "term inal"),
    ] {
        assert!(
            Query::typo_tolerant(query).score(&[text], None).is_none(),
            "overmatched: {query}"
        );
    }
}

#[test]
fn typo_marks_cover_the_word_and_exact_tiers_keep_priority() {
    let query = Query::typo_tolerant("bulling");
    assert_eq!(query.marks("🦀 add stripe billing"), vec![16..23]);
    let fuzzy = query.score(&["billing"], None).unwrap();
    let exact = query.score(&["b-u-l-l-i-n-g"], None).unwrap();
    assert!(exact < fuzzy);
    assert_eq!(
        Query::typo_tolerant("confug").marks("configuration"),
        vec![0..13]
    );
    assert!(
        Query::typo_tolerant("stripe bulling")
            .score(&["Stripe", "billing"], None)
            .is_some()
    );
}
