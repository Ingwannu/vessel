//! Real actor ordering against the fake harness; no model or socket.

use support::{Fixture, Log, prompt, structured, title, within};
use vessel_proto::{SessionEvent, TitleOrigin};
use vessel_session::{SessionError, TitleUpdate};

#[cfg(test)]
#[path = "support/runtime.rs"]
#[expect(
    dead_code,
    reason = "only the shared runtime fixture's title operations are needed"
)]
mod support;

#[tokio::test]
async fn a_checked_generated_title_is_published_and_typed_words_then_win() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        let handle = fixture.start(structured(Vec::new())).await;
        let mut log = Log::of(&handle);
        handle.send(prompt("plain")).await.unwrap();
        log.until(|event| {
            matches!(
                event,
                SessionEvent::Renamed {
                    origin: TitleOrigin::Given,
                    ..
                }
            )
        })
        .await;
        let applied = handle
            .apply_generated_title(title("plain"), "Generated words".parse().unwrap())
            .await
            .unwrap();
        assert_eq!(applied, TitleUpdate::Applied);
        log.until(|event| {
            matches!(
                event,
                SessionEvent::Renamed {
                    origin: TitleOrigin::Generated,
                    ..
                }
            )
        })
        .await;
        assert_eq!(handle.info().title, Some(title("Generated words")));
        assert_eq!(handle.info().title_origin, TitleOrigin::Generated);
        let kept = handle
            .apply_generated_title(title("Generated words"), "Second attempt".parse().unwrap())
            .await
            .unwrap();
        assert_eq!(kept, TitleUpdate::Kept);
        let renamed = handle.rename(title("plain")).await.unwrap();
        assert_eq!(renamed.title_origin, TitleOrigin::Typed);
        assert_eq!(
            handle
                .apply_generated_title(title("plain"), "Too late".parse().unwrap())
                .await
                .unwrap(),
            TitleUpdate::Kept
        );
        assert_eq!(handle.info().title, Some(title("plain")));
        assert_eq!(handle.info().title_origin, TitleOrigin::Typed);
        handle.kill();
        let _end = handle.ended().await;
        assert!(matches!(
            handle
                .apply_generated_title(title("plain"), "Ended".parse().unwrap())
                .await,
            Err(SessionError::Ended)
        ));
    }))
    .await;
}

#[derive(Clone, Copy)]
enum FirstPoll {
    Typed,
    Generated,
}

#[tokio::test]
async fn simultaneous_typed_and_generated_requests_always_finish_typed() {
    within(Box::pin(async {
        let fixture = Fixture::new();
        for first in [FirstPoll::Typed, FirstPoll::Generated] {
            let handle = fixture.start(structured(Vec::new())).await;
            let mut log = Log::of(&handle);
            handle.send(prompt("plain")).await.unwrap();
            log.until(|event| {
                matches!(
                    event,
                    SessionEvent::Renamed {
                        origin: TitleOrigin::Given,
                        ..
                    }
                )
            })
            .await;
            let typed = handle.rename(title("plain"));
            let generated =
                handle.apply_generated_title(title("plain"), "Generated words".parse().unwrap());
            let (typed, generated) = match first {
                FirstPoll::Typed => tokio::join!(typed, generated),
                FirstPoll::Generated => {
                    let (generated, typed) = tokio::join!(generated, typed);
                    (typed, generated)
                }
            };
            let renamed = typed.unwrap();
            assert_eq!(renamed.title_origin, TitleOrigin::Typed);
            assert!(matches!(
                generated.unwrap(),
                TitleUpdate::Applied | TitleUpdate::Kept
            ));
            assert_eq!(handle.info().title, Some(title("plain")));
            assert_eq!(handle.info().title_origin, TitleOrigin::Typed);
            handle.kill();
            let _end = handle.ended().await;
        }
    }))
    .await;
}
