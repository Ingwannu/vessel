//! Single flight, retained overflow, stale results and identity exhaustion.

use super::*;

fn request(value: &Acquired<usize>) -> AcquisitionId {
    match value.acquisition {
        Acquisition::Start(id) | Acquisition::Pending(id) => id,
        Acquisition::Ready(_) => panic!("expected flight"),
    }
}

#[test]
fn retained_consumers_share_one_flight_and_release_idempotently() {
    let mut cache = ImageCache::default();
    let one = cache.acquire(1, Retention::Retained).unwrap();
    let two = cache.acquire(1, Retention::Retained).unwrap();
    assert!(matches!(one.acquisition, Acquisition::Start(_)));
    assert!(matches!(two.acquisition, Acquisition::Pending(_)));
    assert_eq!(request(&one), request(&two));
    assert_eq!(
        cache.complete::<()>(request(&one), Ok(Arc::new(7))),
        Completion::Stored(Arc::new(7))
    );
    assert_eq!(cache.peek(&1), Some(Arc::new(7)));
    assert_eq!(cache.release(one.consumer.unwrap()), Released::Released);
    assert_eq!(
        cache.release(one.consumer.unwrap()),
        Released::AlreadyReleased
    );
    assert_eq!(cache.release(two.consumer.unwrap()), Released::Released);
}

#[test]
fn pinned_overflow_is_soft_and_release_evicts_the_oldest_idle_entry() {
    let mut cache = ImageCache::<usize, usize>::default();
    let mut acquired = Vec::new();
    for key in 0..=IMAGE_CACHE_CAPACITY {
        acquired.push(cache.acquire(key, Retention::Retained).unwrap());
    }
    assert_eq!(cache.count().get(), IMAGE_CACHE_CAPACITY.saturating_add(1));
    let first = acquired.first().unwrap();
    assert_eq!(cache.release(first.consumer.unwrap()), Released::Released);
    assert_eq!(cache.count().get(), IMAGE_CACHE_CAPACITY);
    assert_eq!(
        cache.complete::<()>(request(first), Ok(Arc::new(9))),
        Completion::Uncached(Arc::new(9))
    );
    assert_eq!(cache.peek(&0), None);
}

#[test]
fn a_peek_updates_lru_and_failed_or_evicted_flights_cannot_overwrite_retries() {
    let mut cache = ImageCache::<usize, usize>::default();
    let old = cache.acquire(0, Retention::Transient).unwrap();
    for key in 1..=IMAGE_CACHE_CAPACITY {
        let new = cache.acquire(key, Retention::Transient).unwrap();
        let _done = cache.complete::<()>(request(&new), Ok(Arc::new(key)));
    }
    let current = cache.acquire(0, Retention::Retained).unwrap();
    assert_ne!(request(&old), request(&current));
    assert_eq!(
        cache.complete::<()>(request(&old), Ok(Arc::new(99))),
        Completion::Uncached(Arc::new(99))
    );
    assert_eq!(
        cache.peek(&0),
        None,
        "old result cannot overwrite the new pending read"
    );
    let joined = cache.acquire(0, Retention::Transient).unwrap();
    assert_eq!(request(&joined), request(&current));
    assert_eq!(
        cache.complete(request(&current), Err("failed")),
        Completion::<usize, _>::Failed("failed")
    );
    assert_eq!(
        cache.release(current.consumer.unwrap()),
        Released::AlreadyReleased
    );
    let retry = cache.acquire(0, Retention::Transient).unwrap();
    assert_ne!(request(&retry), request(&current));
    let _done = cache.complete::<()>(request(&retry), Ok(Arc::new(100)));
    assert_eq!(cache.peek(&0), Some(Arc::new(100)));
    assert_eq!(cache.peek(&2), Some(Arc::new(2)));
    let _new = cache
        .acquire(IMAGE_CACHE_CAPACITY.saturating_add(1), Retention::Transient)
        .unwrap();
    assert_eq!(
        cache.peek(&2),
        Some(Arc::new(2)),
        "peek made this entry recent"
    );
    assert_eq!(
        cache.peek(&3),
        None,
        "the oldest idle entry was evicted instead"
    );
}

#[test]
fn exhausting_identity_reservation_leaves_no_partial_entry_or_consumer() {
    let mut cache = ImageCache::<usize, usize> {
        next: u64::MAX.saturating_sub(1),
        ..ImageCache::default()
    };
    assert_eq!(
        cache.acquire(1, Retention::Retained),
        Err(CacheError::IdentityExhausted)
    );
    assert_eq!(cache.count().get(), 0);
    assert!(cache.consumers.is_empty());
    assert!(cache.pending.is_empty());
}
