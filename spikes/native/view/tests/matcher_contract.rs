//! Public consumer contract, using Paseo's provider-catalog-search fixtures.
//! Matching stays in the production shared module. The fixture names are
//! ASCII, so this example's lowercase tie break makes no locale-collation
//! claim; the provider catalog owns that rule.

use vessel_app_view::matcher::Query;

const ENTRIES: [(&str, &str); 6] = [
    ("kiro-cli", "Kiro CLI"),
    ("goose", "goose"),
    ("kimi", "Kimi Code CLI"),
    ("gemini", "Gemini CLI"),
    ("qwen-code", "Qwen Code"),
    ("codex-acp", "Codex"),
];

fn titles(query: &str) -> Vec<&'static str> {
    let query = Query::new(query);
    let mut ranked: Vec<_> = ENTRIES
        .into_iter()
        .filter_map(|(id, title)| query.score(&[title, id], None).map(|score| (score, title)))
        .collect();
    ranked.sort_by(|(left_score, left), (right_score, right)| {
        left_score
            .cmp(right_score)
            .then_with(|| left.to_lowercase().cmp(&right.to_lowercase()))
    });
    ranked.into_iter().map(|(_, title)| title).collect()
}

#[test]
fn empty_query_leaves_catalog_ordering_to_the_consumer() {
    assert!(Query::new("  ").is_empty());
    assert_eq!(
        titles("  "),
        [
            "Codex",
            "Gemini CLI",
            "goose",
            "Kimi Code CLI",
            "Kiro CLI",
            "Qwen Code"
        ]
    );
}

#[test]
fn opaque_scores_rank_visible_name_and_id_fields() {
    assert_eq!(titles("cod"), ["Codex", "Kimi Code CLI", "Qwen Code"]);
    assert_eq!(titles("kiro-cli"), ["Kiro CLI"]);
}

#[test]
fn catalog_tokens_and_subsequences_use_the_command_matcher() {
    assert_eq!(titles("gmni"), ["Gemini CLI"]);
    assert_eq!(titles("qw cod"), ["Qwen Code"]);
}
