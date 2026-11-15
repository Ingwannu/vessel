//! Label priorities, exact rounding and partial metadata are independent of I/O.

use super::*;
use crate::localization::{ENGLISH, LanguagePreference, Translator, resolve_language};

fn completed(tokens: Option<u64>) -> Compaction {
    Compaction {
        status: CompactionStatus::Completed,
        trigger: None,
        pre_tokens: tokens.map(TokenCount::new),
    }
}

#[test]
fn triggers_take_priority_and_zero_is_not_a_reported_token_caption() {
    assert_eq!(
        CompactionCaption::of(&completed(None)),
        CompactionCaption::Completed
    );
    assert_eq!(
        CompactionCaption::of(&completed(Some(0))),
        CompactionCaption::Completed
    );
    for trigger in [CompactionTrigger::Auto, CompactionTrigger::Manual] {
        let mut item = Compaction {
            trigger: Some(trigger),
            ..completed(Some(9000))
        };
        assert_eq!(
            CompactionCaption::of(&item),
            match trigger {
                CompactionTrigger::Auto => CompactionCaption::Auto,
                CompactionTrigger::Manual => CompactionCaption::Manual,
            }
        );
        item.status = CompactionStatus::Loading;
        assert_eq!(CompactionCaption::of(&item), CompactionCaption::Loading);
    }
}

#[test]
fn rounding_matches_positive_javascript_half_up_without_overflow() {
    for (tokens, expected) in [
        (1, 0),
        (499, 0),
        (500, 1),
        (1499, 1),
        (1500, 2),
        (u64::MAX, 18_446_744_073_709_552),
    ] {
        let CompactionCaption::Tokens(rounded) = CompactionCaption::of(&completed(Some(tokens)))
        else {
            panic!("positive token count must have a token caption");
        };
        assert_eq!(rounded.get(), expected);
    }
}

#[test]
fn completion_retains_unreported_loading_metadata_and_replaces_reported_values() {
    let loading = Compaction {
        status: CompactionStatus::Loading,
        trigger: Some(CompactionTrigger::Auto),
        pre_tokens: Some(TokenCount::new(9000)),
    };
    let merged = merge_compaction(&loading, &completed(None));
    assert_eq!(merged.status, CompactionStatus::Completed);
    assert_eq!(merged.trigger, loading.trigger);
    assert_eq!(merged.pre_tokens, loading.pre_tokens);
    let replacement = Compaction {
        trigger: Some(CompactionTrigger::Manual),
        ..completed(Some(0))
    };
    assert_eq!(merge_compaction(&loading, &replacement), replacement);
}

#[test]
fn whole_marker_sentences_match_the_reference_in_both_languages() {
    let en = Translator::get(ENGLISH).unwrap().timeline_items();
    let ko = Translator::get(resolve_language(
        LanguagePreference::System,
        Some("ko"),
        None,
    ))
    .unwrap()
    .timeline_items();
    assert_eq!(
        en.compaction(&completed(Some(1500))),
        "Context compacted (\u{2068}2\u{2069}K tokens)"
    );
    assert_eq!(
        ko.compaction(&completed(Some(1500))),
        "컨텍스트가 압축되었습니다 (\u{2068}2\u{2069}K 토큰)"
    );
    assert_eq!(en.compaction(&completed(None)), "Context compacted");
    assert_eq!(ko.compaction(&completed(None)), "컨텍스트가 압축되었습니다");
}
