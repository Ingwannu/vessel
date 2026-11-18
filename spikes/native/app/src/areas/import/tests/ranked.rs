//! A ranked host (TRACKER 790, ADR 0109): every agent read at once with
//! Paseo's limit, the prompt preview under each title, and the search sent
//! to the node once it rests for Paseo's 200 ms.
use super::*;

fn prompted(id: &str, minute: i64, prompt: Option<&str>) -> Entry {
    Entry {
        title: None,
        first_prompt: prompt.map(str::to_owned),
        ..entry(id, minute)
    }
}

fn ranked_of(ask: &Ask) -> &vessel_app_view::history::Ranked {
    let Ask::List {
        ranked: Some(ranked),
        cursor: None,
        ..
    } = ask
    else {
        panic!("a ranked listing: {ask:?}");
    };
    ranked
}

#[gpui::test]
fn a_ranked_host_is_asked_its_limit_and_rows_show_the_prompt_preview(cx: &mut TestAppContext) {
    let (shell, cx, mut commands) = shell(cx, vec![ranked_machine("laptop", &[codex(), claude()])]);
    open(&shell, cx);
    let asks = sent(&mut commands);
    assert_eq!(asks.len(), 2, "one read per agent");
    for ask in &asks {
        let ranked = ranked_of(ask);
        assert_eq!(ranked.limit, 15);
        assert_eq!(ranked.query, None);
    }
    answer(&shell, cx, asks, |harness| {
        if harness == &codex() {
            vec![prompted("with", 2, Some("fix the build"))]
        } else {
            vec![prompted("without", 1, None)]
        }
    });
    for selector in [
        "history-preview-codex-with",
        "history-preview-claude-code-without",
    ] {
        assert!(cx.debug_bounds(selector).is_some(), "{selector} is drawn");
    }
    shell.read_with(cx, |shell, _cx| {
        let rows = shell.areas.import.model.rows(&shell.areas.import.query);
        let with = rows
            .iter()
            .find(|row| row.entry.id.as_str() == "with")
            .unwrap();
        assert_eq!(with.entry.preview(), Some("fix the build"));
        assert_eq!(
            with.entry.shown_title(),
            Some("fix the build"),
            "titled by its prompt"
        );
        let without = rows
            .iter()
            .find(|row| row.entry.id.as_str() == "without")
            .unwrap();
        assert_eq!(without.entry.preview(), None, "\"No prompt preview\"");
    });
}

#[gpui::test]
fn a_search_reaches_the_node_once_it_rests(cx: &mut TestAppContext) {
    let (shell, cx, mut commands) = shell(cx, vec![ranked_machine("laptop", &[codex(), claude()])]);
    open(&shell, cx);
    let asks = sent(&mut commands);
    answer(&shell, cx, asks, |_| {
        vec![prompted("native", 1, Some("fix"))]
    });
    shell.update(cx, |shell, cx| shell.history_search("bui", cx));
    shell.update(cx, |shell, cx| shell.history_search("build", cx));
    cx.run_until_parked();
    assert!(
        sent(&mut commands).is_empty(),
        "nothing before the search rests"
    );
    cx.executor().advance_clock(SEARCH_SETTLE);
    cx.run_until_parked();
    let asks = sent(&mut commands);
    assert_eq!(asks.len(), 2, "one read per agent, for the last words only");
    for ask in &asks {
        let ranked = ranked_of(ask);
        assert_eq!(ranked.query.as_deref(), Some("build"));
        assert_eq!(ranked.limit, 15);
    }
}

#[gpui::test]
fn an_older_host_keeps_the_cursor_listing_and_its_client_search(cx: &mut TestAppContext) {
    let (shell, cx, mut commands) = shell(cx, vec![machine("laptop", &[codex()])]);
    open(&shell, cx);
    let asks = sent(&mut commands);
    assert!(matches!(asks.as_slice(), [Ask::List { ranked: None, .. }]));
    answer(&shell, cx, asks, |_| vec![entry("native", 1)]);
    shell.update(cx, |shell, cx| shell.history_search("nothing", cx));
    cx.executor().advance_clock(SEARCH_SETTLE);
    cx.run_until_parked();
    assert!(sent(&mut commands).is_empty(), "filtered here, not asked");
    assert!(cx.debug_bounds("history-preview-codex-native").is_none());
}
