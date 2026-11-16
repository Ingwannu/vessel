//! Work limits, independent of scheduler latency on the shared build VM.

use super::{FindCache, Work};
use crate::timeline::find::{Finder, SLICE, find_all};
use crate::timeline::{Entry, EntryKey, EntryKind, Prompt};
use vessel_proto::EventSeq;

const ENTRIES: usize = 5_000;
const REPEATS: usize = 20;
const PARAGRAPH: &str = "The **build** failed in `crates/vessel-web` because the test named \
    `a_qr_code_is_an_svg` expected a header. I changed [the check](https://x.test) \
    and ran the suite again; everything passes now. ";

fn entries() -> Vec<Entry> {
    (0..ENTRIES)
        .map(|index| {
            let seq = EventSeq::new(u64::try_from(index).unwrap());
            Entry {
                key: EntryKey::Record(seq),
                seq,
                at: chrono::DateTime::UNIX_EPOCH,
                kind: if index % 2 == 0 {
                    EntryKind::Prompt(Prompt {
                        text: format!("Please look at item {index} and fix what is broken."),
                        replayed: false,
                        turn: 1,
                        attachments: Vec::new(),
                    })
                } else {
                    EntryKind::Message {
                        text: PARAGRAPH.repeat(8),
                    }
                },
                rev: 0,
            }
        })
        .collect()
}

#[test]
fn find_work_is_linear_cached_and_bounded_per_frame() {
    // The old wall-clock assertions confused host preemption with search work.
    // Count the expensive cache rebuild and every entry visit at their actual
    // call sites; identical output alone would miss a cache or slice regression.
    let mut entries = entries();
    let mut cache = FindCache::default();
    let found = find_all(&mut cache, &entries, "suite again");
    assert_eq!(found.len(), 2_500 * 8);
    assert_eq!(
        cache.work,
        Work {
            visits: ENTRIES,
            folds: ENTRIES
        }
    );
    for _ in 0..REPEATS {
        let before = cache.work;
        assert_eq!(find_all(&mut cache, &entries, "suite again"), found);
        assert_eq!(cache.work.visits - before.visits, ENTRIES);
        assert_eq!(cache.work.folds, before.folds, "warm finds must not refold");
    }
    // Query changes reuse the same readable/folded text, too.
    let before = cache.work;
    assert_eq!(find_all(&mut cache, &entries, "BUILD").len(), 2_500 * 8);
    assert_eq!(cache.work.folds, before.folds);
    entries[1].rev += 1;
    assert_eq!(find_all(&mut cache, &entries, "suite again"), found);
    assert_eq!(cache.work.folds - before.folds, 1, "only the changed entry");

    let mut fresh = FindCache::default();
    let mut finder = Finder::new("suite again");
    let steps = ENTRIES.div_ceil(SLICE);
    assert_eq!(SLICE, 200, "keep the page's frame work budget");
    for frame in 0..steps {
        let before = fresh.work;
        let result = finder.step(&mut fresh, &entries);
        let visited = fresh.work.visits - before.visits;
        assert_eq!(visited, SLICE.min(ENTRIES - frame * SLICE));
        assert!(fresh.work.folds - before.folds <= SLICE);
        if frame + 1 == steps {
            assert_eq!(result, Some(found.clone()));
        } else {
            assert_eq!(result, None, "yield between frames");
        }
    }
    assert_eq!(
        fresh.work,
        Work {
            visits: ENTRIES,
            folds: ENTRIES
        }
    );
    let before = fresh.work;
    assert_eq!(
        Finder::new("  ").step(&mut fresh, &entries),
        Some(Vec::new())
    );
    assert_eq!(fresh.work, before, "empty queries perform no entry work");
}
