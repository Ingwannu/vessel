//! Real synthetic secret edits and loopback preparation; no account credentials.
use super::*;
use crate::{
    dictation::{audio::Segment, endpoint::tests::Server},
    fleet::FleetFiles,
};
use axum::{Router, http::HeaderMap, routing::post};
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use vessel_proto::{
    SecretName,
    dictation::{DictationChange, DictationPcm, EndpointBase, SecretReference},
};
use vessel_store::DataDir;

const TEST_LIMIT: Duration = Duration::from_secs(60);
fn fixture() -> (tempfile::TempDir, FleetFiles, Factory, SecretName) {
    let temp = tempfile::tempdir().unwrap();
    let files = FleetFiles::new(&DataDir::new(temp.path().join("data")));
    let factory = Factory::new(Reader::new(files.clone()));
    (temp, files, factory, "TRANSCRIPTION".parse().unwrap())
}
fn endpoint(name: &SecretName, base: EndpointBase) -> SetDictation {
    SetDictation {
        provider: Some(DictationChange::Set(DictationProvider::Endpoint)),
        endpoint_base: Some(DictationChange::Set(base)),
        key: Some(DictationChange::Set(SecretReference::from(name.clone()))),
        ..SetDictation::default()
    }
}
fn prepared(result: Result<Source, PrepareError>) -> super::super::endpoint::Endpoint {
    match result.unwrap() {
        Source::Endpoint(endpoint) => endpoint,
        Source::Fake(_) => panic!("real preparation unexpectedly selected a fake"),
    }
}

#[tokio::test]
async fn every_preparation_reads_the_selected_key_fresh_and_keeps_its_own_client() {
    tokio::time::timeout(TEST_LIMIT, async {
        let seen = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&seen);
        let server = Server::start(Router::new().route(
            "/v1/audio/transcriptions",
            post(move |headers: HeaderMap| {
                let count = Arc::clone(&count);
                async move {
                    let selected = match count.fetch_add(1, Ordering::SeqCst) {
                        0 => "synthetic-first",
                        1 => "synthetic-second",
                        _ => panic!("unexpected upload"),
                    };
                    assert!(
                        headers.get("authorization").unwrap().as_bytes()
                            == format!("Bearer {selected}").as_bytes(),
                        "preparation retained a stale selected credential"
                    );
                    "{\"text\":\"recognized\"}"
                }
            }),
        ))
        .await;
        let (_temp, files, factory, name) = fixture();
        let saved = endpoint(&name, server.base());
        crate::fleet::set_secret(&files, &name, "synthetic-first").unwrap();
        let first = prepared(
            factory
                .prepare(
                    &saved,
                    crate::dictation::quota::tests::lease(),
                    &CancellationToken::new(),
                )
                .await,
        );
        crate::fleet::set_secret(&files, &name, "synthetic-second").unwrap();
        let second = prepared(
            factory
                .prepare(
                    &saved,
                    crate::dictation::quota::tests::lease(),
                    &CancellationToken::new(),
                )
                .await,
        );
        let segment =
            Segment::from_test_pcm(DictationPcm::new(1000_i16.to_le_bytes().to_vec()).unwrap());
        assert!(first.transcribe(&segment).await.unwrap().as_str() == "recognized");
        assert!(second.transcribe(&segment).await.unwrap().as_str() == "recognized");
        assert_eq!(seen.load(Ordering::SeqCst), 2);
        assert!(crate::fleet::delete_secret(&files, &name).unwrap());
        assert!(matches!(
            factory
                .prepare(
                    &saved,
                    crate::dictation::quota::tests::lease(),
                    &CancellationToken::new()
                )
                .await,
            Err(PrepareError::KeyAbsent)
        ));
        server.finish().await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn defaults_disabled_and_unknown_choices_refuse_without_reading_the_secret_store() {
    tokio::time::timeout(TEST_LIMIT, async {
        let (_temp, files, factory, _name) = fixture();
        std::fs::create_dir_all(files.secrets().parent().unwrap()).unwrap();
        std::fs::write(files.secrets(), "broken = [synthetic-private").unwrap();
        let local = factory
            .prepare(
                &SetDictation::default(),
                crate::dictation::quota::tests::lease(),
                &CancellationToken::new(),
            )
            .await;
        match local {
            Err(PrepareError::LocalNotBuilt { model }) => {
                assert_eq!(model, LocalModelId::default_model());
            }
            Ok(_)
            | Err(
                PrepareError::Configuration(_)
                | PrepareError::Disabled
                | PrepareError::KeyAbsent
                | PrepareError::Secret(_)
                | PrepareError::Endpoint(_),
            ) => panic!("local preparation selected another provider"),
        }
        let off = SetDictation {
            switch: Some(DictationChange::Set(DictationSwitch::Off)),
            ..SetDictation::default()
        };
        assert!(matches!(
            factory
                .prepare(
                    &off,
                    crate::dictation::quota::tests::lease(),
                    &CancellationToken::new()
                )
                .await,
            Err(PrepareError::Disabled)
        ));
        let unknown = SetDictation {
            provider: Some(DictationChange::Set(DictationProvider::Unknown)),
            ..SetDictation::default()
        };
        assert!(matches!(
            factory
                .prepare(
                    &unknown,
                    crate::dictation::quota::tests::lease(),
                    &CancellationToken::new()
                )
                .await,
            Err(PrepareError::Configuration(
                DictationDecodeError::UnknownProvider
            ))
        ));
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn absent_blank_and_invalid_header_keys_never_become_a_provider() {
    tokio::time::timeout(TEST_LIMIT, async {
        let (_temp, files, factory, name) = fixture();
        let saved = endpoint(&name, EndpointBase::openai());
        assert!(matches!(
            factory
                .prepare(
                    &saved,
                    crate::dictation::quota::tests::lease(),
                    &CancellationToken::new()
                )
                .await,
            Err(PrepareError::KeyAbsent)
        ));
        crate::fleet::set_secret(&files, &name, " \t\n ").unwrap();
        assert!(matches!(
            factory
                .prepare(
                    &saved,
                    crate::dictation::quota::tests::lease(),
                    &CancellationToken::new()
                )
                .await,
            Err(PrepareError::KeyAbsent)
        ));
        let marker = "synthetic-invalid\nheader";
        crate::fleet::set_secret(&files, &name, marker).unwrap();
        let Err(error) = factory
            .prepare(
                &saved,
                crate::dictation::quota::tests::lease(),
                &CancellationToken::new(),
            )
            .await
        else {
            panic!("a multiline credential became a provider");
        };
        assert!(matches!(
            error,
            PrepareError::Endpoint(ProviderError::Header(_))
        ));
        assert!(std::error::Error::source(&error).is_some());
        for rendered in [format!("{error}"), format!("{error:?}")] {
            assert!(!rendered.contains(marker));
            assert!(!rendered.contains(name.as_str()));
        }
    })
    .await
    .unwrap();
}
