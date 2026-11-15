//! Admission, ownership, serial replacement and actual RPC outlet closure.
use super::test_fixture::*;
use super::*;
use crate::dictation::updates::End;
use vessel_proto::dictation::{DictationChange, DictationReason, DictationSwitch};

#[tokio::test]
async fn every_operation_checks_connection_identity_and_replacement_cannot_steal_it() {
    within(async {
        let f = Fixture::new().await;
        let first = Link::new().await;
        let other = Link::new().await;
        let key = id("same-token");
        f.registry
            .start(&first.caller, key.clone(), format())
            .await
            .unwrap();
        assert!(matches!(
            refusal(f.registry.start(&other.caller, key.clone(), format()).await).as_ref(),
            RegistryError::Taken
        ));
        assert!(matches!(
            refusal(
                f.registry
                    .chunk(&other.caller, &key, ChunkSeq::new(0), pcm())
                    .await
            )
            .as_ref(),
            RegistryError::Forbidden
        ));
        assert!(matches!(
            refusal(
                f.registry
                    .finish(&other.caller, &key, ChunkSeq::new(0))
                    .await
            )
            .as_ref(),
            RegistryError::Forbidden
        ));
        assert!(matches!(
            refusal(f.registry.next(&other.caller, &key).await).as_ref(),
            RegistryError::Forbidden
        ));
        assert!(matches!(
            refusal(f.registry.cancel(&other.caller, &key).await).as_ref(),
            RegistryError::Forbidden
        ));
        assert_eq!(
            f.registry
                .chunk(&first.caller, &key, ChunkSeq::new(0), pcm())
                .await
                .unwrap(),
            Some(ChunkSeq::new(0))
        );
        f.registry.cancel(&first.caller, &key).await.unwrap();
        first.close().await;
        other.close().await;
        f.finish().await;
    })
    .await;
}

#[tokio::test]
async fn connection_reservations_are_shared_by_aliases_and_consumed_terminals_release_them() {
    within(async {
        let f = Fixture::new().await;
        let link = Link::new().await;
        let delegated = link.context.acting_as(crate::principal::Principal::Owner);
        assert_eq!(delegated.life().owner(), link.context.life().owner());
        let alias = delegated.dictation_context().unwrap().caller().clone();
        let first = id("first"); let second = id("second"); let third = id("third");
        f.registry.start(&link.caller, first.clone(), format()).await.unwrap();
        f.registry.start(&alias, second.clone(), format()).await.unwrap();
        let error = refusal(f.registry.start(&alias, third.clone(), format()).await);
        assert!(matches!(error.as_ref(), RegistryError::Quota(QuotaError::Connection(_))));
        let error = refusal(f.registry.chunk(&alias, &first, ChunkSeq::new(vessel_proto::dictation::MAX_SEQ.get().checked_add(1).unwrap()), pcm()).await);
        assert!(matches!(error.as_ref(), RegistryError::Stream(source) if source.reason() == Some(DictationReason::TooLong)));
        // Selecting failure does not free an unread terminal reservation.
        assert!(matches!(refusal(f.registry.start(&alias, third.clone(), format()).await).as_ref(), RegistryError::Quota(QuotaError::Connection(_))));
        let entry = f.registry.entry(&alias, &first).await.unwrap();
        match f.registry.next(&alias, &first).await.unwrap() {
            Event::End(End::Failed(source)) => assert_eq!(source.reason(), Some(DictationReason::TooLong)),
            Event::Partial(_) | Event::End(End::Final(_)) => panic!("expected the selected failure"),
        }
        entry.finished.cancelled().await;
        f.registry.start(&alias, third.clone(), format()).await.unwrap();
        f.registry.cancel(&alias, &second).await.unwrap(); f.registry.cancel(&alias, &third).await.unwrap();
        // Cancellation follows Paseo's cleanup: a missing id is already cancelled.
        f.registry.cancel(&alias, &first).await.unwrap();
        link.close().await; f.finish().await;
    }).await;
}

#[tokio::test]
async fn the_node_bound_cannot_be_bypassed_by_opening_more_connections() {
    within(async {
        let f = Fixture::new().await;
        let mut links = Vec::new();
        for _ in 0..super::super::quota::NODE_STREAMS {
            links.push(Link::new().await);
        }
        for (index, link) in links.iter().enumerate() {
            f.registry
                .start(&link.caller, id(&format!("stream-{index}")), format())
                .await
                .unwrap();
        }
        let extra = Link::new().await;
        assert!(matches!(
            refusal(
                f.registry
                    .start(&extra.caller, id("overflow"), format())
                    .await
            )
            .as_ref(),
            RegistryError::Quota(QuotaError::Node(_))
        ));
        f.registry
            .cancel(&links[0].caller, &id("stream-0"))
            .await
            .unwrap();
        f.registry
            .start(&extra.caller, id("overflow"), format())
            .await
            .unwrap();
        f.registry.shutdown().await;
        for link in links {
            link.close().await;
        }
        extra.close().await;
        f.finish().await;
    })
    .await;
}

#[tokio::test]
async fn replacement_tears_down_old_before_reading_current_settings_and_failed_preparation_keeps_it_gone()
 {
    within(async {
        let f = Fixture::new().await;
        let link = Link::new().await;
        let key = id("replace");
        f.registry
            .start(&link.caller, key.clone(), format())
            .await
            .unwrap();
        let entry = f.registry.entry(&link.caller, &key).await.unwrap();
        let old = f.registry.handle(&link.caller, &key).await.unwrap();
        let mut held = f.settings.write().await;
        let mut replacing = Box::pin(f.registry.start(&link.caller, key.clone(), format()));
        assert!(futures_util::poll!(replacing.as_mut()).is_pending());
        old.closed.cancelled().await;
        assert!(matches!(
            refusal(f.registry.handle(&link.caller, &key).await).as_ref(),
            RegistryError::Conflict
        ));
        held.switch = Some(DictationChange::Set(DictationSwitch::Off));
        drop(held);
        assert!(matches!(
            replacing.await.unwrap_err().as_ref(),
            RegistryError::Prepare(PrepareError::Disabled)
        ));
        entry.finished.cancelled().await;
        assert!(matches!(
            refusal(f.registry.entry(&link.caller, &key).await).as_ref(),
            RegistryError::Gone
        ));
        link.close().await;
        f.finish().await;
    })
    .await;
}

#[tokio::test]
async fn a_full_replacement_mailbox_changes_neither_the_first_stop_nor_the_pending_format() {
    within(async {
        let f = Fixture::new().await; let link = Link::new().await; let key = id("mailbox");
        let held = f.settings.write().await;
        let mut first = Box::pin(f.registry.start(&link.caller, key.clone(), format()));
        assert!(futures_util::poll!(first.as_mut()).is_pending());
        let entry = f.registry.entry(&link.caller, &key).await.unwrap();
        let mut second = Box::pin(f.registry.start(&link.caller, key.clone(), format()));
        assert!(futures_util::poll!(second.as_mut()).is_pending());
        let third = refusal(f.registry.start(&link.caller, key.clone(), Pcm16Mono::new(vessel_proto::dictation::SampleRate::MAX)).await);
        assert!(matches!(third.as_ref(), RegistryError::Mailbox(_)));
        let control = entry.phase.borrow().control();
        let mut selected = control.subscribe();
        while selected.borrow().is_none() { selected.changed().await.unwrap(); }
        assert_eq!(selected.borrow().as_ref().unwrap().reason(), Some(DictationReason::Replaced));
        drop(held);
        assert!(matches!(first.await.unwrap_err().as_ref(), RegistryError::Stream(source) if source.reason() == Some(DictationReason::Replaced)));
        second.await.unwrap();
        let second_bytes = usize::try_from(format().rate().get()).unwrap().checked_mul(size_of::<i16>()).unwrap();
        let chunk = DictationPcm::new(vec![0; second_bytes]).unwrap();
        assert_eq!(f.registry.chunk(&link.caller, &key, ChunkSeq::new(0), chunk).await.unwrap(), Some(ChunkSeq::new(0)));
        let following = DictationPcm::new(vec![0; second_bytes]).unwrap();
        assert_eq!(f.registry.chunk(&link.caller, &key, ChunkSeq::new(1), following).await.unwrap(), Some(ChunkSeq::new(1)));
        f.registry.cancel(&link.caller, &key).await.unwrap();
        link.close().await; f.finish().await;
    }).await;
}

#[tokio::test]
async fn actual_outlet_close_retires_streams_even_while_the_lifetime_guard_is_retained() {
    within(async {
        let f = Fixture::new().await;
        let link = Link::new().await;
        let other = Link::new().await;
        let key = id("closing");
        let sibling = id("still-open");
        f.registry
            .start(&link.caller, key.clone(), format())
            .await
            .unwrap();
        f.registry
            .start(&other.caller, sibling.clone(), format())
            .await
            .unwrap();
        let entry = f.registry.entry(&link.caller, &key).await.unwrap();
        let mut waiting = Box::pin(f.registry.next(&link.caller, &key));
        assert!(futures_util::poll!(waiting.as_mut()).is_pending());
        link.close_transport();
        match waiting.await.unwrap() {
            Event::End(End::Failed(source)) => {
                assert_eq!(source.reason(), Some(DictationReason::Cancelled));
            }
            Event::Partial(_) | Event::End(End::Final(_)) => {
                panic!("transport closure produced text")
            }
        }
        entry.finished.cancelled().await;
        // The real context is still retained; physical close must also reach
        // consumers that know only the canonical life accessor.
        link.context.life().closed().cancelled().await;
        assert_eq!(
            f.registry
                .chunk(&other.caller, &sibling, ChunkSeq::new(0), pcm())
                .await
                .unwrap(),
            Some(ChunkSeq::new(0))
        );
        f.registry.cancel(&other.caller, &sibling).await.unwrap();
        link.close().await;
        other.close().await;
        f.finish().await;
    })
    .await;
}

#[tokio::test]
async fn stale_cleanup_cannot_remove_the_new_record_for_the_same_id() {
    within(async {
        let f = Fixture::new().await;
        let link = Link::new().await;
        let key = id("same-id");
        f.registry
            .start(&link.caller, key.clone(), format())
            .await
            .unwrap();
        let old = f.registry.entry(&link.caller, &key).await.unwrap();
        f.registry.cancel(&link.caller, &key).await.unwrap();
        f.registry
            .start(&link.caller, key.clone(), format())
            .await
            .unwrap();
        let current = f.registry.entry(&link.caller, &key).await.unwrap();
        assert!(!Arc::ptr_eq(&old, &current));
        {
            let mut entries = f.registry.state.entries.lock().await;
            remove_current(&mut entries, &key, &old);
        }
        assert_eq!(
            f.registry
                .chunk(&link.caller, &key, ChunkSeq::new(0), pcm())
                .await
                .unwrap(),
            Some(ChunkSeq::new(0))
        );
        f.registry.cancel(&link.caller, &key).await.unwrap();
        link.close().await;
        f.finish().await;
    })
    .await;
}

fn refusal<T>(result: Result<T>) -> Arc<RegistryError> {
    match result {
        Err(error) => error,
        Ok(_) => panic!("the operation unexpectedly succeeded"),
    }
}

#[tokio::test]
async fn replacement_uses_the_fresh_secret_and_a_failed_new_start_leaves_no_old_stream() {
    within(async {
        let f = Fixture::new().await;
        let link = Link::new().await;
        let key = id("fresh-key");
        f.registry
            .start(&link.caller, key.clone(), format())
            .await
            .unwrap();
        let entry = f.registry.entry(&link.caller, &key).await.unwrap();
        let old = f.registry.handle(&link.caller, &key).await.unwrap();
        assert!(crate::fleet::delete_secret(&f.files, &f.name).unwrap());
        assert!(matches!(
            refusal(f.registry.start(&link.caller, key.clone(), format()).await).as_ref(),
            RegistryError::Prepare(PrepareError::KeyAbsent)
        ));
        old.closed.cancelled().await;
        entry.finished.cancelled().await;
        crate::fleet::set_secret(&f.files, &f.name, "synthetic-new-key").unwrap();
        f.registry
            .start(&link.caller, key.clone(), format())
            .await
            .unwrap();
        f.registry.cancel(&link.caller, &key).await.unwrap();
        link.close().await;
        f.finish().await;
    })
    .await;
}

#[tokio::test]
async fn the_saved_snapshot_reads_the_existing_settings_owner_without_a_second_cache() {
    within(async {
        let f = Fixture::new().await; let link = Link::new().await;
        let settings = crate::settings::test_fixture::open(vessel_store::DataDir::new(f.data.path()));
        let registry = Registry::new(Factory::new(crate::dictation::secret_read::Reader::new(f.files.clone())), Snapshot::Saved(settings), &CancellationToken::new());
        assert!(matches!(refusal(registry.start(&link.caller, id("default-local"), format()).await).as_ref(), RegistryError::Prepare(PrepareError::LocalNotBuilt { model }) if *model == vessel_proto::dictation::LocalModelId::default_model()));
        registry.shutdown().await; link.close().await; f.finish().await;
    }).await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_cancelled_start_keeps_preparing_capacity_until_teardown_and_cannot_publish_an_orphan() {
    within(async {
        let f = Fixture::new().await;
        let link = Link::new().await;
        let first_id = id("cancelled-start");
        let held = f.settings.write().await;
        let mut first = Box::pin(f.registry.start(&link.caller, first_id.clone(), format()));
        assert!(futures_util::poll!(first.as_mut()).is_pending());
        let entry = f.registry.entry(&link.caller, &first_id).await.unwrap();
        let mut phase = entry.phase.subscribe();
        // On this cooperative runtime, publication continues into the blocked
        // snapshot read before this awakened observer can resume.
        phase.changed().await.unwrap();
        drop(first);
        let control = phase.borrow().control();
        let mut selected = control.subscribe();
        while selected.borrow().is_none() {
            selected.changed().await.unwrap();
        }
        assert_eq!(
            selected.borrow().as_ref().unwrap().reason(),
            Some(DictationReason::Cancelled)
        );
        assert!(!entry.finished.is_cancelled());
        let second_id = id("sibling-start");
        let mut second = Box::pin(f.registry.start(&link.caller, second_id.clone(), format()));
        assert!(futures_util::poll!(second.as_mut()).is_pending());
        assert!(matches!(
            refusal(
                f.registry
                    .start(&link.caller, id("too-many"), format())
                    .await
            )
            .as_ref(),
            RegistryError::Quota(QuotaError::Connection(_))
        ));
        drop(held);
        entry.finished.cancelled().await;
        second.await.unwrap();
        assert!(matches!(
            refusal(f.registry.entry(&link.caller, &first_id).await).as_ref(),
            RegistryError::Gone
        ));
        assert!(!link.context.life().closed().is_cancelled());
        assert_eq!(
            f.registry
                .chunk(&link.caller, &second_id, ChunkSeq::new(0), pcm())
                .await
                .unwrap(),
            Some(ChunkSeq::new(0))
        );
        f.registry.cancel(&link.caller, &second_id).await.unwrap();
        link.close().await;
        f.finish().await;
    })
    .await;
}

#[tokio::test]
async fn only_the_last_service_handle_drop_closes_its_records_and_does_not_close_the_connection() {
    within(async {
        let f = Fixture::new().await;
        let link = Link::new().await;
        let key = id("service-drop");
        let registry = Registry::new(
            Factory::new(crate::dictation::secret_read::Reader::new(f.files.clone())),
            Snapshot::Given(Arc::clone(&f.settings)),
            &CancellationToken::new(),
        );
        let alias = registry.clone();
        registry
            .start(&link.caller, key.clone(), format())
            .await
            .unwrap();
        let entry = registry.entry(&link.caller, &key).await.unwrap();
        drop(registry);
        assert!(!entry.finished.is_cancelled());
        assert_eq!(
            alias
                .chunk(&link.caller, &key, ChunkSeq::new(0), pcm())
                .await
                .unwrap(),
            Some(ChunkSeq::new(0))
        );
        drop(alias);
        entry.finished.cancelled().await;
        assert!(!link.context.life().closed().is_cancelled());
        f.registry
            .start(&link.caller, key.clone(), format())
            .await
            .unwrap();
        f.registry.cancel(&link.caller, &key).await.unwrap();
        link.close().await;
        f.finish().await;
    })
    .await;
}

#[tokio::test]
async fn failed_roots_stay_shared_across_starts_and_service_recreation_recovers() {
    within(async {
        let f = Fixture::new().await;
        let link = Link::new().await;
        let probes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let service = || {
            Registry::new(
                Factory::with_root_probe(
                    crate::dictation::secret_read::Reader::new(f.files.clone()),
                    Arc::clone(&probes),
                ),
                Snapshot::Given(Arc::clone(&f.settings)),
                &CancellationToken::new(),
            )
        };
        let registry = service();
        let first = refusal(
            registry
                .start(&link.caller, id("roots-first"), format())
                .await,
        );
        let second = refusal(
            registry
                .start(&link.caller, id("roots-second"), format())
                .await,
        );
        let root_cause = |error: Arc<RegistryError>| {
            let RegistryError::Prepare(PrepareError::Endpoint(
                crate::dictation::endpoint::ProviderError::Roots(source),
            )) = error.as_ref()
            else {
                panic!("expected the injected native-root refusal");
            };
            Arc::clone(source)
        };
        assert!(Arc::ptr_eq(&root_cause(first), &root_cause(second)));
        assert_eq!(probes.load(std::sync::atomic::Ordering::SeqCst), 1);
        registry.shutdown().await;
        drop(registry);
        let recreated = service();
        let key = id("roots-recovered");
        recreated
            .start(&link.caller, key.clone(), format())
            .await
            .unwrap();
        assert_eq!(probes.load(std::sync::atomic::Ordering::SeqCst), 2);
        recreated.cancel(&link.caller, &key).await.unwrap();
        recreated.shutdown().await;
        link.close().await;
        f.finish().await;
    })
    .await;
}

#[tokio::test]
async fn canonical_life_observes_transport_close_with_retained_contexts_and_aliases() {
    within(async {
        let first = Link::new().await;
        let other = Link::new().await;
        let alias = first.context.acting_as(crate::principal::Principal::Owner);
        let owner = first.context.life().owner();
        let closed = alias.life().closed().child_token();
        assert_eq!(owner, alias.life().owner());
        assert!(!closed.is_cancelled());
        first.close_transport();
        closed.cancelled().await;
        assert_eq!(owner, first.context.life().owner());
        assert_eq!(owner, alias.life().owner());
        assert!(first.context.life().closed().is_cancelled());
        assert!(!other.context.life().closed().is_cancelled());
        drop(alias);
        first.close().await;
        other.close().await;
    })
    .await;
}
