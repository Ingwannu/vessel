//! Actual first-input/runtime/client/proposal paths with isolated fakes and loopback only.

use super::*;
use crate::models::tests::{Fixture, within};
use bytes::Bytes;
use vessel_proto::{
    ContentBlock, ModelFailure, ModelStop, SessionEvent, SessionInput, TitleOrigin,
    parking::ParkAfter,
    settings::{MetadataProvider, SetNodeSettings},
};
use vessel_testkit::upstream::Reply;

mod physical;

fn context(fixture: &Fixture) -> preparation::Context {
    preparation::Context {
        cwd: fixture.data.path().to_owned(),
        git: None,
        tracked: tokio_util::task::TaskTracker::new().token(),
    }
}

fn manual() -> MetadataSettings {
    MetadataSettings::new(vec![MetadataProvider {
        route: "sonnet".parse().unwrap(),
        model: None,
        effort: None,
    }])
    .unwrap()
}
fn prompt() -> SessionInput {
    SessionInput::Prompt {
        content: vec![ContentBlock::Text {
            text: "plain".into(),
        }],
    }
}
fn reply(title: &str) -> Reply {
    Reply::Status { status: 200, retry_after: None, body: Bytes::from(serde_json::to_vec(&serde_json::json!({
        "id":"msg_fixture", "type":"message", "role":"assistant", "model":"claude-sonnet-4-5",
        "content":[{"type":"text", "text":title}], "stop_reason":"end_turn", "stop_sequence":null,
        "usage":{"input_tokens":17,"output_tokens":7}
    })).unwrap()) }
}
async fn given(handle: &SessionHandle) {
    let mut info = handle.watch_info();
    while info.borrow().title_origin != TitleOrigin::Given {
        info.changed().await.unwrap();
    }
}

async fn until(handle: &SessionHandle, predicate: fn(&SessionEvent) -> bool) {
    let follow = handle.follow(vessel_proto::EventSeq::ZERO).unwrap();
    if follow.cached.iter().any(|record| predicate(&record.event)) {
        return;
    }
    let mut live = follow.live;
    loop {
        if predicate(&live.recv().await.unwrap().event) {
            return;
        }
    }
}
async fn recorded(handle: &SessionHandle) {
    until(handle, |event| matches!(event, SessionEvent::ModelCall(_))).await;
}
async fn idle(handle: &SessionHandle) {
    until(handle, |event| {
        matches!(
            event,
            SessionEvent::Agent {
                event: vessel_proto::AgentEvent::TurnEnded { .. }
            }
        )
    })
    .await;
}

#[tokio::test]
async fn cached_first_input_runs_one_bounded_call_and_records_the_applied_title() {
    within(async {
        let fixture = Fixture::new().await;
        let (handle, records) = fixture.structured().await;
        handle.send(prompt()).await.unwrap();
        given(&handle).await;
        // Subscribe only after the prompt and provisional title; neither may be lost.
        let seed = source::first(&handle).await.unwrap();
        fixture
            .upstream
            .reply(reply(r#"{"title":"Generated work"}"#));
        let client = fixture.models.title_generator.client.as_ref().unwrap();
        let result = driver::generate(
            &fixture.models,
            client,
            &handle,
            &manual(),
            seed,
            context(&fixture),
        )
        .await
        .unwrap();
        assert_eq!(result, driver::Outcome::Applied);
        assert_eq!(handle.info().title_origin, TitleOrigin::Generated);
        let requests = fixture.upstream.seen();
        assert_eq!(requests.len(), 1);
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["stream"], false);
        assert_eq!(body["max_tokens"], prepare::TITLE_OUTPUT_TOKENS);
        recorded(&handle).await;
        handle.kill();
        let _ended = handle.ended().await;
        let records = records.await.unwrap();
        let calls: Vec<_> = records
            .iter()
            .filter_map(|record| {
                if let SessionEvent::ModelCall(call) = &record.event {
                    Some(call)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].stop, ModelStop::EndTurn);
        assert_eq!(calls[0].failure, None);
        assert_eq!(calls[0].usage.input.get(), 17);
        assert!(matches!(
            records.last().unwrap().event,
            SessionEvent::Status { .. }
        ));
        fixture.stop().await;
    })
    .await;
}

#[tokio::test]
async fn invalid_semantic_title_keeps_the_given_title_but_not_the_usage_hidden() {
    within(async {
        let fixture = Fixture::new().await;
        let (handle, records) = fixture.structured().await;
        handle.send(prompt()).await.unwrap();
        given(&handle).await;
        let seed = source::first(&handle).await.unwrap();
        fixture.upstream.reply(reply("not title JSON"));
        let client = fixture.models.title_generator.client.as_ref().unwrap();
        assert!(matches!(
            driver::generate(
                &fixture.models,
                client,
                &handle,
                &manual(),
                seed,
                context(&fixture)
            )
            .await,
            Err(driver::GenerateError::Text(_))
        ));
        assert_eq!(handle.info().title_origin, TitleOrigin::Given);
        recorded(&handle).await;
        handle.kill();
        let _ended = handle.ended().await;
        let records = records.await.unwrap();
        let call = records
            .iter()
            .find_map(|record| {
                if let SessionEvent::ModelCall(call) = &record.event {
                    Some(call)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(call.stop, ModelStop::EndTurn);
        assert_eq!(call.usage.output.get(), 7);
        assert_eq!(call.failure, Some(ModelFailure::Truncated));
        assert_eq!(fixture.upstream.seen().len(), 1);
        fixture.stop().await;
    })
    .await;
}

#[derive(Clone, Copy)]
enum Cancel {
    Typed,
    Parked,
    Ended,
    Node,
}

async fn cancel_title(handle: &SessionHandle, stop: &CancellationToken, cancel: Cancel) {
    match cancel {
        Cancel::Typed => {
            let _renamed = handle.rename("plain".parse().unwrap()).await.unwrap();
        }
        Cancel::Parked => {
            idle(handle).await;
            let _parked = handle.park().await.unwrap();
        }
        Cancel::Ended => {
            handle.kill();
            let _ended = handle.ended().await;
        }
        Cancel::Node => stop.cancel(),
    }
}

#[tokio::test]
async fn identical_typed_words_parking_ending_and_node_stop_cancel_inflight_work() {
    within(async {
        for cancel in [Cancel::Typed, Cancel::Parked, Cancel::Ended, Cancel::Node] {
            let fixture = Fixture::new().await;
            let (handle, records) = fixture.structured().await;
            handle.send(prompt()).await.unwrap();
            given(&handle).await;
            let dir = vessel_store::DataDir::new(fixture.data.path());
            let settings = Arc::new(crate::settings::Settings::open(
                dir,
                Arc::clone(&fixture.runtime),
                ParkAfter::Never,
                false,
            ));
            let _current = settings
                .change(SetNodeSettings {
                    metadata: Some(manual()),
                    ..SetNodeSettings::default()
                })
                .await
                .unwrap();
            let stop = CancellationToken::new();
            let task = task::Task {
                handle: handle.clone(),
                models: Arc::clone(&fixture.models),
                settings,
                cwd: fixture.data.path().to_owned(),
                git: None,
                stop: stop.clone(),
                work: tokio_util::task::TaskTracker::new().token(),
            };
            let (client, entered) = client::Client::stalled();
            let running = tokio::spawn(async move {
                task.run(&client).await;
            });
            entered.notified().await;
            cancel_title(&handle, &stop, cancel).await;
            running.await.unwrap();
            assert_ne!(handle.info().title_origin, TitleOrigin::Generated);
            handle.kill();
            let _ended = handle.ended().await;
            let records = records.await.unwrap();
            assert!(matches!(
                records.last().unwrap().event,
                SessionEvent::Status { .. }
            ));
            assert!(!records.iter().any(|record| matches!(
                &record.event,
                SessionEvent::Renamed {
                    origin: TitleOrigin::Generated,
                    ..
                }
            )));
            fixture.stop().await;
        }
    })
    .await;
}

#[tokio::test]
async fn later_file_metadata_survives_an_unrelated_patch_and_reopen() {
    within(async {
        let fixture = Fixture::new().await;
        let dir = vessel_store::DataDir::new(fixture.data.path());
        let raw = serde_json::to_vec(&serde_json::json!({
            "park_after": ParkAfter::Never,
            "metadata": {"future":true,"providers":[{"route":"sonnet","future":true,"effort":"future"}]}
        })).unwrap();
        std::fs::write(dir.settings_path(), &raw).unwrap();
        let settings = crate::settings::Settings::open(dir.clone(), Arc::clone(&fixture.runtime), ParkAfter::Never, false);
        let metadata = settings.metadata().await;
        assert_eq!(metadata.providers()[0].effort, Some(vessel_proto::settings::MetadataEffort::Unknown));
        assert!(fixture.models.validate_metadata(&metadata).is_err());
        let changed = settings.change(SetNodeSettings {
            drivers: Some(vessel_proto::settings::SetAgentDrivers::default()),
            ..SetNodeSettings::default()
        }).await.unwrap();
        assert_eq!(changed.metadata.as_ref(), Some(&metadata));
        let copies: Vec<_> = std::fs::read_dir(dir.root()).unwrap().map(|entry| entry.unwrap().path())
            .filter(|path| path.file_name().unwrap().to_string_lossy().starts_with("settings.json.later-")).collect();
        assert_eq!(copies.len(), 1);
        assert_eq!(std::fs::read(&copies[0]).unwrap(), raw);
        let reopened = crate::settings::Settings::open(dir, Arc::clone(&fixture.runtime), ParkAfter::Never, false);
        assert_eq!(reopened.metadata().await, metadata);
        fixture.stop().await;
    }).await;
}

#[tokio::test]
async fn a_live_subscription_waits_for_its_first_given_title_and_typed_words_stop_it() {
    within(async {
        let fixture = Fixture::new().await;
        let (handle, records) = fixture.structured().await;
        let (seed, sent) = tokio::join!(source::first(&handle), handle.send(prompt()));
        sent.unwrap();
        let seed = seed.unwrap();
        assert_eq!(seed.expected.as_str(), "plain");
        let _renamed = handle.rename(seed.expected.clone()).await.unwrap();
        source::lost(&handle, &seed.expected).await;
        assert!(matches!(
            source::first(&handle).await,
            Err(source::SourceError::Kept)
        ));
        assert!(fixture.upstream.seen().is_empty());
        handle.kill();
        let _ended = handle.ended().await;
        let _records = records.await.unwrap();
        fixture.stop().await;
    })
    .await;
}

#[tokio::test]
async fn parking_then_waking_before_polling_cannot_restore_generation_ownership() {
    within(async {
        let fixture = Fixture::new().await;
        let (handle, records) = fixture.structured().await;
        handle.send(prompt()).await.unwrap();
        given(&handle).await;
        let seed = source::first(&handle).await.unwrap();
        idle(&handle).await;
        let _parked = handle.park().await.unwrap();
        let _woken = handle.wake().await.unwrap();
        assert!(
            handle.accepts_generated_title(&seed.expected),
            "current info alone misses the intervening park"
        );
        source::lost(&handle, &seed.expected).await;
        assert!(fixture.upstream.seen().is_empty());
        handle.kill();
        let _ended = handle.ended().await;
        let _records = records.await.unwrap();
        fixture.stop().await;
    })
    .await;
}
