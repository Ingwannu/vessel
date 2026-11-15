//! Protocol 9 in the session runtime, with `fake-acp` as Claude's adapter:
//! the options a session logs and changes, and its title.
//!
//! Tests wait on records, never on time; the timeout only turns a hang
//! into a failure.

use vessel_proto::{
    ErrorCode, HarnessKind, SessionEvent, SessionMode, SessionSpec, SessionStatus, TerminalSize,
    TitleOrigin,
};
use vessel_session::SessionError;

use support::{Fixture, Log, mode, prompt, select, structured, title, within};

#[cfg(test)]
#[path = "support/runtime.rs"]
#[expect(
    dead_code,
    reason = "the shared runtime fixture, not every helper of it"
)]
mod support;

/// An untitled session takes the first line of its first prompt; a rename
/// replaces it; later prompts do not.
#[tokio::test]
async fn the_first_prompt_names_a_session_and_a_rename_replaces_it() {
    // Boxed: it holds several session infos across awaits, and every field
    // a protocol adds to them grows this future past clippy's bound.
    within(Box::pin(async {
        let fixture = Fixture::new();
        let handle = fixture.start(structured(Vec::new())).await;
        assert_eq!(handle.info().title, None);
        assert_eq!(handle.info().title_origin, TitleOrigin::Unknown);
        let mut log = Log::of(&handle);
        // The fake ends a turn at once only for `plain`; the first line
        // and the cut are the protocol's own tests (`SessionTitle`).
        handle.send(prompt("plain")).await.unwrap();
        log.until(|event| matches!(event, SessionEvent::Renamed { origin: TitleOrigin::Given, .. }))
            .await;
        assert_eq!(handle.info().title, Some(title("plain")));
        assert_eq!(handle.info().title_origin, TitleOrigin::Given);
        log.turn_over().await;

        let renamed = handle.rename(title("Release notes")).await.unwrap();
        assert_eq!(renamed.title, Some(title("Release notes")));
        assert_eq!(renamed.title_origin, TitleOrigin::Typed);
        handle.send(prompt("plain")).await.unwrap();
        log.turn_over().await;
        log.until(|event| matches!(event, SessionEvent::Renamed { title, .. } if title.as_str() == "Release notes"))
            .await;
        assert_eq!(log.titles(), ["plain", "Release notes"]);
        assert_eq!(handle.info().title, Some(title("Release notes")));

        handle.kill();
        let _end = handle.ended().await;
        let error = handle.rename(title("Too late")).await.unwrap_err();
        assert!(matches!(error, SessionError::Ended), "{error:?}");
    }))
    .await;
}

/// A session created with a title keeps it: its prompts do not rename it.
#[tokio::test]
async fn a_typed_title_is_kept() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        let handle = fixture
            .start(structured(Vec::new()).with_title(title("Mine")))
            .await;
        assert_eq!(handle.info().title, Some(title("Mine")));
        assert_eq!(handle.info().title_origin, TitleOrigin::Typed);
        let mut log = Log::of(&handle);
        handle.send(prompt("plain")).await.unwrap();
        log.turn_over().await;
        assert!(log.titles().is_empty(), "{:?}", log.titles());
        handle.kill();
        let _end = handle.ended().await;
    }))
    .await;
}

/// Fork creation carries a derived title without pretending a person
/// supplied it; the ordinary rename path still records a typed title.
#[tokio::test]
async fn a_launch_can_mark_a_derived_title() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        let handle = fixture
            .start_with(
                structured(Vec::new()).with_title(title("Original (fork)")),
                vessel_session::LaunchOptions {
                    title_origin: Some(TitleOrigin::Given),
                    ..vessel_session::LaunchOptions::default()
                },
            )
            .await;
        assert_eq!(handle.info().title_origin, TitleOrigin::Given);
        assert_eq!(
            handle.rename(title("Mine")).await.unwrap().title_origin,
            TitleOrigin::Typed
        );
        handle.kill();
        let _end = handle.ended().await;
    }))
    .await;
}

/// The options are in the log after the handshake, as chosen; a change is
/// in the log too, and is what the session carries on.
#[tokio::test]
async fn a_session_logs_its_options_and_their_changes() {
    // Boxed: with protocol 11's pull requests, terminals and added agents
    // in a session's state, this test's future passed the lint's size.
    within(Box::pin(async {
        let fixture = Fixture::new();
        let handle = fixture.start(structured(vec![select("mode", "plan")])).await;
        // Before any report, what the spec chose.
        let mut log = Log::of(&handle);
        log.until(|event| matches!(event, SessionEvent::OptionsChanged { .. }))
            .await;
        assert_eq!(log.reports().first().and_then(|options| mode(options)), Some("Plan"));
        assert_eq!(mode(&handle.options().await.unwrap()), Some("Plan"));

        let changed = handle.set_option(select("mode", "default")).await.unwrap();
        assert_eq!(mode(&changed), Some("Default"));
        log.until(|event| matches!(event, SessionEvent::OptionsChanged { options } if mode(options) == Some("Default")))
            .await;
        assert!(handle.choices().contains(&select("mode", "default")));

        let error = handle
            .set_option(select("mode", "locked"))
            .await
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::Conflict, "the agent's refusal");
        let error = handle.set_option(select("mode", "yolo")).await.unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidRequest, "not offered");
        handle.kill();
        let _end = handle.ended().await;
    }))
    .await;
}

/// A choice the agent does not offer fails the session at its start.
#[tokio::test]
async fn a_choice_not_offered_fails_the_session() {
    within(async {
        let fixture = Fixture::new();
        let handle = fixture
            .start(structured(vec![select("mode", "yolo")]))
            .await;
        let end = handle.ended().await;
        let SessionStatus::Failed { message, .. } = &end.status else {
            panic!("failed: {:?}", end.status);
        };
        assert!(
            message.contains("option `mode` has no value `yolo`"),
            "{message}"
        );
    })
    .await;
}

/// A terminal session has no options to read or set, and can be renamed.
#[tokio::test]
async fn a_terminal_session_has_no_options_but_a_title() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        let spec = SessionSpec::new(
            HarnessKind::ClaudeCode,
            SessionMode::Terminal {
                args: Vec::new(),
                size: TerminalSize::default(),
            },
        )
        .with_cwd(std::env::temp_dir());
        let handle = fixture.start(spec).await;
        let error = handle.options().await.unwrap_err();
        assert!(matches!(error, SessionError::NoOptions), "{error:?}");
        assert_eq!(error.code(), ErrorCode::Unsupported);
        assert!(handle.choices().is_empty());
        let renamed = handle.rename(title("A shell")).await.unwrap();
        assert_eq!(renamed.title, Some(title("A shell")));
        handle.kill();
        let _end = handle.ended().await;
    }))
    .await;
}
