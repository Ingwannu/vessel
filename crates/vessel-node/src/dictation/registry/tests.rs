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
        let alias = link.caller.clone();
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

fn refusal<T>(result: Result<T>) -> Arc<RegistryError> {
    match result {
        Err(error) => error,
        Ok(_) => panic!("the operation unexpectedly succeeded"),
    }
}
