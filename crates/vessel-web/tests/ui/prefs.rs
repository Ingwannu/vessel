//! The page's preferences and drafts through the bridge (ADR 0025): where
//! they are kept, in what order changes apply, what every tab is pushed,
//! and what is kept for this launch only.
#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "tests reading the few frames they wait for among the others"
)]

use std::path::Path;
use std::sync::Arc;

use vessel_proto::SessionId;
use vessel_web::wire::{
    CallId, DraftKey, DraftText, DraftsState, Kept, LaunchOnly, MAX_DRAFT, Op, OpFailure,
    OpOutcome, OpResult, PreferenceChange, Preferences, PreferencesState, ServerFrame, Theme,
};

use crate::bridge::op_reply;
use crate::server::{get, http};
use crate::support::{Source, Tab, Ui, within};

/// Ask the bridge `op` and wait for its answer.
async fn ask(tab: &mut Tab, op: Op) -> OpOutcome {
    let id = tab.op(op).await;
    op_reply(tab, id).await
}

/// The preferences an answer carries.
fn state(outcome: OpOutcome) -> Arc<PreferencesState> {
    match outcome {
        OpOutcome::Ok(OpResult::Preferences { preferences } | OpResult::Prefer { preferences }) => {
            preferences
        }
        other => panic!("{other:?}"),
    }
}

/// Every draft an answer carries.
fn drafts(outcome: OpOutcome) -> DraftsState {
    match outcome {
        OpOutcome::Ok(OpResult::Drafts { drafts }) => drafts,
        other => panic!("{other:?}"),
    }
}

/// The next preferences pushed to `tab` at `revision` or later.
async fn pushed(tab: &mut Tab, revision: u64) -> Arc<PreferencesState> {
    tab.until(|frame| match frame {
        ServerFrame::Preferences { preferences } if preferences.revision.get() >= revision => {
            Some(preferences)
        }
        _other => None,
    })
    .await
}

async fn prefer(tab: &mut Tab, change: PreferenceChange) -> Arc<PreferencesState> {
    state(ask(tab, Op::Prefer { change }).await)
}

fn theme(state: &PreferencesState) -> Theme {
    state.values.appearance.theme
}

fn this_launch(state: &PreferencesState) -> &LaunchOnly {
    match &state.kept {
        Kept::ThisLaunch { why } => why,
        Kept::File { path } => panic!("kept in {path}"),
    }
}

#[tokio::test]
async fn a_change_is_written_answered_whole_and_pushed_to_every_tab() {
    within(Box::pin(async {
        let dir = tempfile::tempdir().unwrap();
        let ui = Ui::start(Source::default().keeping_in(dir.path())).await;
        let mut one = ui.tab().await;
        let mut two = ui.tab().await;
        let first = pushed(&mut two, 1).await;

        let read = state(ask(&mut one, Op::Preferences).await);
        assert_eq!(read.values, Preferences::default(), "defaults on disk");
        let file = Source::preferences_in(dir.path());
        assert_eq!(
            read.kept,
            Kept::File {
                path: file.display().to_string()
            }
        );
        assert!(!file.exists(), "nothing written before a change");

        // The answer, and the push the tab that asked is sent too: they
        // leave by two paths, so either may come first.
        let id = one
            .op(Op::Prefer {
                change: PreferenceChange::Theme(Theme::Dark),
            })
            .await;
        let mut answer = None;
        let mut own_push = None;
        while answer.is_none() || own_push.is_none() {
            match one.next().await {
                ServerFrame::OpReply {
                    id: replied,
                    outcome,
                } if replied == id => {
                    answer = Some(state(outcome));
                }
                ServerFrame::Preferences { preferences } if theme(&preferences) == Theme::Dark => {
                    own_push = Some(preferences);
                }
                _other => {}
            }
        }
        let changed = answer.unwrap();
        assert_eq!(theme(&changed), Theme::Dark);
        assert!(changed.revision > first.revision);
        assert_eq!(own_push.unwrap(), changed);
        let seen = pushed(&mut two, changed.revision.get()).await;
        assert_eq!(*seen, *changed);

        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("\"theme\": \"dark\""), "{text}");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode(&file), 0o600);
            assert_eq!(mode(file.parent().unwrap()), 0o700);
        }
        // A new tab starts from it.
        let mut three = ui.tab().await;
        let first = pushed(&mut three, 1).await;
        assert_eq!(theme(&first), Theme::Dark);
    }))
    .await;
}

#[tokio::test]
async fn changes_from_one_tab_apply_and_are_answered_in_order() {
    within(Box::pin(async {
        let dir = tempfile::tempdir().unwrap();
        let ui = Ui::start(Source::default().keeping_in(dir.path())).await;
        let mut tab = ui.tab().await;
        let changes = [Theme::Light, Theme::Dark, Theme::System, Theme::Dark];
        let mut ids = Vec::new();
        for theme in changes {
            let change = PreferenceChange::Theme(theme);
            ids.push(tab.op(Op::Prefer { change }).await);
        }
        let mut answered: Vec<(CallId, Theme, u64)> = Vec::new();
        while answered.len() < changes.len() {
            if let ServerFrame::OpReply { id, outcome } = tab.next().await {
                let state = state(outcome);
                answered.push((id, theme(&state), state.revision.get()));
            }
        }
        let order: Vec<CallId> = answered.iter().map(|(id, _theme, _revision)| *id).collect();
        assert_eq!(order, ids, "answered in the order sent");
        let themes: Vec<Theme> = answered
            .iter()
            .map(|(_id, theme, _revision)| *theme)
            .collect();
        assert_eq!(themes, changes, "each applied after the one before");
        assert!(answered.windows(2).all(|pair| pair[0].2 < pair[1].2));
        let text = std::fs::read_to_string(Source::preferences_in(dir.path())).unwrap();
        assert!(text.contains("\"theme\": \"dark\""), "{text}");
    }))
    .await;
}

#[tokio::test]
async fn a_change_another_process_made_is_published_on_the_next_read() {
    within(Box::pin(async {
        let dir = tempfile::tempdir().unwrap();
        let ui = Ui::start(Source::default().keeping_in(dir.path())).await;
        let mut one = ui.tab().await;
        let mut two = ui.tab().await;
        let before = prefer(&mut one, PreferenceChange::TitleCount(false)).await;
        let file = Source::preferences_in(dir.path());
        // Another `vessel ui` writes the file meanwhile.
        std::fs::write(
            &file,
            r#"{"version":1,"appearance":{"theme":"light"},"notifications":{"title":false}}"#,
        )
        .unwrap();
        let read = state(ask(&mut one, Op::Preferences).await);
        assert_eq!(theme(&read), Theme::Light);
        assert!(read.revision > before.revision);
        let seen = pushed(&mut two, read.revision.get()).await;
        assert_eq!(theme(&seen), Theme::Light);
        // A change keeps what the other process wrote.
        let after = prefer(&mut one, PreferenceChange::TitleCount(true)).await;
        assert_eq!(theme(&after), Theme::Light);
        assert!(after.values.notifications.title);
        // Reading again with nothing changed publishes nothing new.
        let again = state(ask(&mut one, Op::Preferences).await);
        assert_eq!(again.revision, after.revision);
    }))
    .await;
}

/// A change kept for this launch: answered, pushed, and never written.
async fn kept_in_memory(ui: &Ui, why: impl Fn(&LaunchOnly) -> bool) {
    let mut tab = ui.tab().await;
    let read = state(ask(&mut tab, Op::Preferences).await);
    assert!(why(this_launch(&read)), "{:?}", read.kept);
    let changed = prefer(&mut tab, PreferenceChange::Theme(Theme::Dark)).await;
    assert_eq!(theme(&changed), Theme::Dark);
    assert!(why(this_launch(&changed)), "{:?}", changed.kept);
    let again = state(ask(&mut tab, Op::Preferences).await);
    assert_eq!(theme(&again), Theme::Dark, "kept in memory");
}

#[tokio::test]
async fn without_a_place_they_are_kept_for_this_launch() {
    within(Box::pin(async {
        let ui = Ui::start(Source::default()).await;
        kept_in_memory(&ui, |why| *why == LaunchOnly::NoPlace).await;

        let ui = Ui::start(Source::default().not_knowing_where()).await;
        kept_in_memory(&ui, |why| {
            matches!(why, LaunchOnly::NoPath { message } if message.contains("no home directory"))
        })
        .await;
    }))
    .await;
}

#[tokio::test]
async fn an_unreadable_or_newer_file_is_used_for_this_launch_and_never_written() {
    within(Box::pin(async {
        let dir = tempfile::tempdir().unwrap();
        let file = Source::preferences_in(dir.path());
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        let shown = file.display().to_string();

        std::fs::write(&file, "not json").unwrap();
        let ui = Ui::start(Source::default().keeping_in(dir.path())).await;
        kept_in_memory(&ui, |why| {
            matches!(why, LaunchOnly::Unreadable { path, message }
                if *path == shown && message.contains("not a preferences file"))
        })
        .await;
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "not json");

        let newer = r#"{"version":2,"appearance":{"theme":"light","interface_size":16}}"#;
        std::fs::write(&file, newer).unwrap();
        let ui = Ui::start(Source::default().keeping_in(dir.path())).await;
        let mut tab = ui.tab().await;
        let read = state(ask(&mut tab, Op::Preferences).await);
        assert_eq!(theme(&read), Theme::Light, "read for this launch");
        assert_eq!(read.values.appearance.interface_size.get(), 16);
        kept_in_memory(
            &ui,
            |why| matches!(why, LaunchOnly::Newer { path, version: 2 } if *path == shown),
        )
        .await;
        assert_eq!(std::fs::read_to_string(&file).unwrap(), newer);

        // A file that turns bad in the middle of a launch moves the
        // preferences to memory from then on, and every tab is told.
        std::fs::remove_file(&file).unwrap();
        let ui = Ui::start(Source::default().keeping_in(dir.path())).await;
        let mut one = ui.tab().await;
        let mut two = ui.tab().await;
        let _first = prefer(&mut one, PreferenceChange::Theme(Theme::Light)).await;
        std::fs::write(&file, "[]").unwrap();
        let moved = prefer(&mut one, PreferenceChange::Theme(Theme::Dark)).await;
        assert!(matches!(this_launch(&moved), LaunchOnly::Unreadable { .. }));
        assert_eq!(theme(&moved), Theme::Dark);
        let seen = pushed(&mut two, moved.revision.get()).await;
        assert!(matches!(this_launch(&seen), LaunchOnly::Unreadable { .. }));
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "[]");
    }))
    .await;
}

fn draft_text(text: &str) -> DraftText {
    DraftText::try_from(text.to_owned()).unwrap()
}

async fn save(tab: &mut Tab, key: DraftKey, text: &str) {
    let outcome = ask(
        tab,
        Op::SaveDraft {
            key,
            text: draft_text(text),
        },
    )
    .await;
    assert!(
        matches!(outcome, OpOutcome::Ok(OpResult::SaveDraft)),
        "{outcome:?}"
    );
}

async fn one_draft(tab: &mut Tab, key: DraftKey) -> Option<String> {
    match ask(tab, Op::Draft { key }).await {
        OpOutcome::Ok(OpResult::Draft { text }) => text.map(|text| text.as_str().to_owned()),
        other => panic!("{other:?}"),
    }
}

fn texts(state: &DraftsState) -> Vec<&str> {
    state
        .drafts
        .iter()
        .map(|draft| draft.text.as_str())
        .collect()
}

#[tokio::test]
async fn drafts_are_saved_read_back_and_merged_across_two_bridges() {
    within(Box::pin(async {
        let dir = tempfile::tempdir().unwrap();
        let first = Ui::start(Source::default().keeping_in(dir.path())).await;
        let second = Ui::start(Source::default().keeping_in(dir.path())).await;
        let mut one = first.tab().await;
        let mut two = second.tab().await;
        let session = DraftKey::Session {
            session: SessionId::random(),
        };

        save(&mut one, DraftKey::New, "a new idea").await;
        // Read back (the first save of a quiet second is written before
        // the next op is taken).
        assert_eq!(
            one_draft(&mut one, DraftKey::New).await.as_deref(),
            Some("a new idea")
        );
        save(&mut two, session, "half a prompt").await;
        let both = drafts(ask(&mut two, Op::Drafts).await);
        assert_eq!(texts(&both), ["a new idea", "half a prompt"]);
        let file = Source::drafts_in(dir.path());
        assert_eq!(
            both.kept,
            Kept::File {
                path: file.display().to_string()
            }
        );
        // The first bridge reads the second's draft from the file.
        let seen = drafts(ask(&mut one, Op::Drafts).await);
        assert_eq!(texts(&seen), ["a new idea", "half a prompt"]);
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(
            text.contains("half a prompt") && text.contains("a new idea"),
            "{text}"
        );

        // An empty save removes; a read in the same second merges what is
        // not written yet.
        save(&mut one, DraftKey::New, "").await;
        assert_eq!(one_draft(&mut one, DraftKey::New).await, None);
        assert_eq!(
            texts(&drafts(ask(&mut one, Op::Drafts).await)),
            ["half a prompt"]
        );
    }))
    .await;
}

#[tokio::test]
async fn drafts_past_the_count_lose_the_oldest() {
    within(Box::pin(async {
        let ui = Ui::start(Source::default()).await;
        let mut tab = ui.tab().await;
        let keys: Vec<DraftKey> = (0..65)
            .map(|_| DraftKey::Session {
                session: SessionId::random(),
            })
            .collect();
        for key in &keys {
            save(&mut tab, *key, "words").await;
        }
        let kept = drafts(ask(&mut tab, Op::Drafts).await);
        assert_eq!(kept.drafts.len(), 64);
        assert_eq!(
            kept.kept,
            Kept::ThisLaunch {
                why: LaunchOnly::NoPlace
            }
        );
        let oldest = keys.first().unwrap();
        assert!(kept.drafts.iter().all(|draft| draft.key != *oldest));
        assert_eq!(one_draft(&mut tab, *oldest).await, None);
        assert_eq!(
            one_draft(&mut tab, *keys.last().unwrap()).await.as_deref(),
            Some("words")
        );
    }))
    .await;
}

#[tokio::test]
async fn a_draft_past_its_bound_is_refused_as_unreadable() {
    within(Box::pin(async {
        let ui = Ui::start(Source::default()).await;
        let mut tab = ui.tab().await;
        let long = "x".repeat(MAX_DRAFT.checked_add(1).unwrap());
        let id = tab
            .frame_text(&format!(
                r#"{{"type":"op","id":{{id}},"op":{{"op":"save_draft","key":{{"kind":"new"}},"text":"{long}"}}}}"#
            ))
            .await;
        match op_reply(&mut tab, id).await {
            OpOutcome::Err { reason, .. } => assert_eq!(reason, OpFailure::Unreadable),
            other @ OpOutcome::Ok(_) => panic!("{other:?}"),
        }
        assert_eq!(one_draft(&mut tab, DraftKey::New).await, None);
    }))
    .await;
}

#[tokio::test]
async fn the_index_carries_the_appearance_after_a_change() {
    within(Box::pin(async {
        let ui = Ui::start(Source::default()).await;
        let mut tab = ui.tab().await;
        let (_code, _headers, before) = http(ui.address, &get("/", &ui.host(), "")).await;
        let _dark = prefer(&mut tab, PreferenceChange::Theme(Theme::Dark)).await;
        let (code, _headers, after) = http(ui.address, &get("/sessions", &ui.host(), "")).await;
        assert_eq!(code, 200);
        if after.contains("The web UI was not built") {
            // The placeholder page is served as it is.
            assert_eq!(after, before);
            assert!(!after.contains("data-theme"));
        } else {
            assert!(
                after.contains(
                    "<html lang=\"en\" data-theme=\"dark\" data-ui-size=\"14\" \
                     data-code-size=\"12\" data-content-width=\"comfortable\""
                ),
                "{after}"
            );
            assert!(!before.contains("data-theme"), "{before}");
        }
    }))
    .await;
}
