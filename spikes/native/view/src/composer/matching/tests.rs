//! Paseo's own cases (`packages/protocol/src/search/text-match.test.ts`),
//! those of its exact tiers, its subsequence and `scoreTextFields`.

use super::*;

fn tier_and_offset(query: &str, text: &str) -> Option<(usize, usize)> {
    score_match(query, text).map(|score| (score.tier.rank(), score.offset))
}

#[test]
fn an_empty_query_and_an_exact_match_are_tier_zero_whatever_the_case() {
    assert_eq!(tier_and_offset("", "anything"), Some((0, 0)));
    assert_eq!(tier_and_offset("pi", "pi"), Some((0, 0)));
    assert_eq!(tier_and_offset("PI", "pi"), Some((0, 0)));
    assert_eq!(tier_and_offset("Pi", "pI"), Some((0, 0)));
}

#[test]
fn whole_words_prefixes_word_starts_and_substrings_rank_in_that_order() {
    assert_eq!(tier_and_offset("feat", "feat/pi-direct-sdk"), Some((1, 0)));
    assert_eq!(tier_and_offset("pi", "feat/pi-direct-sdk"), Some((1, 5)));
    assert_eq!(tier_and_offset("pi", "a b pi c d"), Some((1, 4)));
    assert_eq!(tier_and_offset("par", "party"), Some((2, 0)));
    assert_eq!(tier_and_offset("par", "a/party"), Some((3, 2)));
    assert_eq!(tier_and_offset("art", "party"), Some((4, 1)));
}

#[test]
fn one_query_word_is_not_assembled_across_whitespace() {
    for separator in [" ", "\t", "\n", "\u{a0}"] {
        assert_eq!(
            score_match("terminal", &format!("term{separator}inal")),
            None,
            "{separator:?}"
        );
    }
}

#[test]
fn subsequences_stay_within_a_word() {
    let trmnl = score_match("trmnl", "Fix terminal resizing").unwrap();
    assert_eq!((trmnl.tier.rank(), trmnl.offset, trmnl.spread), (5, 4, 8));
    assert_eq!(
        score_match("abc", "AgentBrowserConfig").map(|score| score.tier.rank()),
        Some(5)
    );
    assert_eq!(
        score_match("pasbab", "paseo-babysit").map(|score| score.tier.rank()),
        Some(5)
    );
    assert_eq!(
        score_match("confg", "check configuration").map(|score| score.offset),
        Some(6)
    );
    assert_eq!(score_match("xyz", "feat/pi-direct-sdk"), None);
}

#[test]
fn the_best_occurrence_counts_and_the_earlier_on_a_tie() {
    assert_eq!(tier_and_offset("ab", "xabxab-yy"), Some((4, 1)));
    assert_eq!(tier_and_offset("ab", "xxab x-ab-y"), Some((1, 7)));
    assert_eq!(tier_and_offset("pi", "pi-a pi-b"), Some((1, 0)));
}

#[test]
fn common_separators_are_word_boundaries() {
    for text in ["x/pi", "x-pi", "x_pi", "x pi", "x.pi", "x:pi"] {
        assert_eq!(tier_and_offset("pi", text), Some((1, 2)), "{text}");
    }
    assert_eq!(tier_and_offset("202", "#202 feat"), Some((1, 1)));
    let title = "#202 feat(server): replace Pi ACP with direct SDK provider";
    assert_eq!(tier_and_offset("pi", title).map(|(tier, _)| tier), Some(1));
    assert_eq!(tier_and_offset("202", title), Some((1, 1)));
    assert_eq!(
        tier_and_offset("replace", title).map(|(tier, _)| tier),
        Some(1)
    );
}

#[test]
fn every_token_must_match_some_field() {
    assert!(score_fields("stripe billing", &["add stripe billing system"]).is_some());
    assert!(score_fields("stripe rosetta", &["add stripe billing system"]).is_none());
    assert!(score_fields("stripe main", &["add stripe billing", "main"]).is_some());
    assert!(score_fields("stipe bulling", &["add stripe billing"]).is_none());
    assert!(score_fields("labdes", &["Label as Design"]).is_none());
    assert!(score_fields("lbl dsgn", &["Label as Design"]).is_some());
}

#[test]
fn scores_sort_by_tier_then_offset_then_spread() {
    let prefix = score_match("com", "commit").unwrap();
    let word_start = score_match("com", "paseo-committee").unwrap();
    assert!(prefix < word_start);
    assert!(score_fields("comm", &["commit"]) < score_fields("comm", &["paseo-committee"]));
}
