//! Served line counts for `workspace.list` and `workspace.get` (ADR 0120).
//!
//! A folder a served client has asked about is counted again every
//! [`CACHE`], and dropped [`INTEREST`] after the last request. At most
//! [`LIMIT`] counts run at once, under the shared git pool. A count that
//! finds git busy or out of time keeps the last count. At most [`MAX_SLOTS`]
//! folders. Both sides zero, no comparison, and a folder that is not a
//! git checkout are stored as none. The generation moves only when a
//! stored count changes, including the first time it is stored.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use tokio::sync::{OwnedSemaphorePermit, Semaphore, watch};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_proto::registry::DiffStat;
use vessel_proto::workspace::WorkspaceRoot;
use vessel_workspace::GitError;

use super::Workspaces;

mod count;

/// How often an interested folder is counted again.
const CACHE: Duration = Duration::from_secs(15);
/// How long a folder is counted after the last list or get.
const INTEREST: Duration = Duration::from_secs(60);
/// Counts running at once.
///
/// The shared git pool has eight local slots. Four leaves room for a
/// person's status, diff, or commit. Paseo counts four at a time.
const LIMIT: usize = 4;
/// The most folders counted at once.
const MAX_SLOTS: usize = 1000;

#[derive(Debug)]
struct Slot {
    epoch: u64,
    roots: Vec<WorkspaceRoot>,
    seen: bool,
    stat: Option<DiffStat>,
    asked: Instant,
}

#[derive(Debug, Default)]
struct Book {
    generation: u64,
    next_epoch: u64,
    slots: HashMap<PathBuf, Slot>,
}

impl Book {
    /// Store `stat` for `key`. The new generation when it changed.
    fn publish(&mut self, key: &Path, stat: Option<DiffStat>) -> Option<u64> {
        let slot = self.slots.get_mut(key)?;
        let changed = !slot.seen || slot.stat != stat;
        slot.stat = stat;
        slot.seen = true;
        if !changed {
            return None;
        }
        // A wrapped generation would look current to a client that already
        // has it. Stop moving it; the stored count is still the latest.
        self.generation = self.generation.checked_add(1)?;
        Some(self.generation)
    }
}

#[derive(Debug)]
struct Shared {
    book: Mutex<Book>,
    changed: watch::Sender<u64>,
    stop: CancellationToken,
    counts: Arc<Semaphore>,
    cache: Duration,
    interest: Duration,
}

/// What one count pass decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answer {
    /// A finished answer. None is a real empty comparison.
    Count(Option<DiffStat>),
    /// Git did not answer. The stored count stays.
    Keep,
    /// Not a git checkout. Stored as none.
    Plain,
}

#[derive(Debug, Clone)]
enum Reader {
    Live(Arc<Workspaces>),
    #[cfg(test)]
    Wait(watch::Receiver<Option<Answer>>),
}

/// The node's served diff stats.
///
/// Count tasks hold [`Workspaces`] and [`Shared`], not this value, so
/// dropping the service can cancel them.
#[derive(Debug)]
pub(crate) struct DiffStats {
    shared: Arc<Shared>,
    tasks: TaskTracker,
    reader: Reader,
}

impl DiffStats {
    /// Counts start on the first request and stop with `workspaces`.
    #[must_use]
    pub(crate) fn start(workspaces: Arc<Workspaces>) -> Self {
        let stop = workspaces.stopping().child_token();
        Self {
            shared: Arc::new(fresh(stop, CACHE, INTEREST)),
            tasks: TaskTracker::new(),
            reader: Reader::Live(workspaces),
        }
    }

    /// Remember interest in each folder and start a count for one that is
    /// new. One key per input, in order, canonical when the folder exists.
    #[must_use]
    pub(crate) async fn note(&self, roots: &[(PathBuf, WorkspaceRoot)]) -> Vec<PathBuf> {
        let owned: Vec<PathBuf> = roots.iter().map(|(path, _root)| path.clone()).collect();
        let fallback = owned.clone();
        let keys = tokio::task::spawn_blocking(move || {
            owned
                .into_iter()
                .map(|path| std::fs::canonicalize(&path).unwrap_or(path))
                .collect::<Vec<_>>()
        })
        .await
        .unwrap_or(fallback);
        self.observe(&keys, roots);
        keys
    }

    /// The generation list and get would serve now.
    #[must_use]
    pub(crate) fn generation(&self) -> u64 {
        lock(&self.shared.book).generation
    }

    /// Whether `key` has been counted. A key with no slot does not wait.
    #[must_use]
    pub(crate) fn ready(&self, key: &Path) -> bool {
        lock(&self.shared.book)
            .slots
            .get(key)
            .is_none_or(|slot| slot.seen)
    }

    /// The stored count. None when it is null or the folder is not counted.
    #[must_use]
    pub(crate) fn stat(&self, key: &Path) -> Option<DiffStat> {
        lock(&self.shared.book)
            .slots
            .get(key)
            .and_then(|slot| slot.stat)
    }

    /// Wakes when the generation moves.
    #[must_use]
    pub(crate) fn subscribe(&self) -> watch::Receiver<u64> {
        self.shared.changed.subscribe()
    }

    /// A count whose answers the test sends, with no git and no workspace.
    #[cfg(test)]
    fn scripted(cache: Duration, interest: Duration) -> (Self, watch::Sender<Option<Answer>>) {
        let (tx, rx) = watch::channel(None);
        let stop = CancellationToken::new();
        (
            Self {
                shared: Arc::new(fresh(stop, cache, interest)),
                tasks: TaskTracker::new(),
                reader: Reader::Wait(rx),
            },
            tx,
        )
    }

    fn observe(&self, keys: &[PathBuf], roots: &[(PathBuf, WorkspaceRoot)]) {
        let mut start = Vec::new();
        {
            let mut book = lock(&self.shared.book);
            for (key, (_path, root)) in keys.iter().zip(roots.iter()) {
                if let Some(slot) = book.slots.get_mut(key) {
                    slot.asked = Instant::now();
                    if !slot.roots.contains(root) {
                        slot.roots.push(*root);
                    }
                    continue;
                }
                if book.slots.len() >= MAX_SLOTS {
                    continue;
                }
                let epoch = book.next_epoch;
                book.next_epoch = book.next_epoch.saturating_add(1);
                let _previous = book.slots.insert(
                    key.clone(),
                    Slot {
                        epoch,
                        roots: vec![*root],
                        seen: false,
                        stat: None,
                        asked: Instant::now(),
                    },
                );
                start.push((key.clone(), epoch));
            }
        }
        for (key, epoch) in start {
            let shared = Arc::clone(&self.shared);
            let reader = self.reader.clone();
            drop(self.tasks.spawn(async move {
                follow(key, epoch, shared, reader).await;
            }));
        }
    }
}

impl Drop for DiffStats {
    fn drop(&mut self) {
        self.shared.stop.cancel();
        let _closed = self.tasks.close();
    }
}

fn fresh(stop: CancellationToken, cache: Duration, interest: Duration) -> Shared {
    let (changed, _receiver) = watch::channel(0);
    Shared {
        book: Mutex::new(Book::default()),
        changed,
        stop,
        counts: Arc::new(Semaphore::new(LIMIT)),
        cache,
        interest,
    }
}

fn lock(book: &Mutex<Book>) -> MutexGuard<'_, Book> {
    book.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Whether list or get waits for a count.
///
/// A delegated caller never waits. The first read waits while a folder
/// has not been counted. A later read waits when it already has this
/// generation.
#[must_use]
pub(crate) fn should_wait(
    changed_from: Option<u64>,
    generation: u64,
    pending: bool,
    delegated: bool,
) -> bool {
    if delegated {
        return false;
    }
    let same = changed_from.is_some_and(|revision| revision == generation);
    same || (changed_from.is_none() && pending)
}

async fn follow(key: PathBuf, epoch: u64, shared: Arc<Shared>, reader: Reader) {
    loop {
        if shared.stop.is_cancelled() || retire(&shared, &key, epoch) {
            break;
        }
        let answer = match root_of(&shared, &key) {
            Some(root) => answer_for(&reader, &shared, root).await,
            None => Answer::Plain,
        };
        remember(&shared, &key, answer);
        if shared.stop.is_cancelled() || retire(&shared, &key, epoch) {
            break;
        }
        let Some(asked) = asked_at(&shared, &key, epoch) else {
            break;
        };
        let Some(pause) = pause_for(asked, shared.cache, shared.interest) else {
            let _done = retire(&shared, &key, epoch);
            break;
        };
        tokio::select! {
            () = shared.stop.cancelled() => break,
            () = tokio::time::sleep(pause) => {}
        }
    }
}

async fn answer_for(reader: &Reader, shared: &Shared, root: WorkspaceRoot) -> Answer {
    match reader {
        Reader::Live(workspaces) => read_root(workspaces, shared, root).await,
        #[cfg(test)]
        Reader::Wait(answers) => scripted_answer(answers.clone()).await,
    }
}

async fn read_root(workspaces: &Workspaces, shared: &Shared, root: WorkspaceRoot) -> Answer {
    let resolved = match workspaces.resolve(root).await {
        Ok(resolved) if resolved.git => resolved,
        Ok(_) => return Answer::Plain,
        Err(error) => {
            tracing::debug!(%error, "diff stat: the workspace folder is not available");
            return Answer::Keep;
        }
    };
    let git = match workspaces.git() {
        Ok(git) => git,
        Err(error) => {
            tracing::debug!(%error, "diff stat: git is not available");
            return Answer::Plain;
        }
    };
    let Some(held) = permit(shared).await else {
        return Answer::Keep;
    };
    let answer = classify(count::stat_in(git, resolved.root.path(), resolved.base.as_ref()).await);
    drop(held);
    answer
}

async fn permit(shared: &Shared) -> Option<OwnedSemaphorePermit> {
    Arc::clone(&shared.counts).acquire_owned().await.ok()
}

fn classify(result: Result<Option<DiffStat>, GitError>) -> Answer {
    match result {
        Ok(stat) => Answer::Count(stat),
        Err(GitError::Unexpected { what: "base" }) => Answer::Count(None),
        Err(error) => {
            tracing::debug!(%error, "diff stat: git did not answer; the last count stays");
            Answer::Keep
        }
    }
}

fn root_of(shared: &Shared, key: &Path) -> Option<WorkspaceRoot> {
    lock(&shared.book)
        .slots
        .get(key)
        .and_then(|slot| slot.roots.last().copied())
}

fn remember(shared: &Shared, key: &Path, answer: Answer) {
    let stat = match answer {
        Answer::Keep => return,
        Answer::Plain => None,
        Answer::Count(stat) => stat,
    };
    let generation = {
        let mut book = lock(&shared.book);
        book.publish(key, stat)
    };
    if let Some(generation) = generation {
        let _previous = shared.changed.send_replace(generation);
    }
}

/// `true` when this task should stop. A newer task for the same folder is
/// left in place.
fn retire(shared: &Shared, key: &Path, epoch: u64) -> bool {
    let mut book = lock(&shared.book);
    let Some(slot) = book.slots.get(key) else {
        return true;
    };
    if slot.epoch != epoch {
        return true;
    }
    if slot.asked.elapsed() < shared.interest {
        return false;
    }
    let _removed = book.slots.remove(key);
    true
}

fn asked_at(shared: &Shared, key: &Path, epoch: u64) -> Option<Instant> {
    let book = lock(&shared.book);
    let slot = book.slots.get(key)?;
    (slot.epoch == epoch).then_some(slot.asked)
}

/// How long to wait before the next count.
///
/// None when interest has already ended, so the task retires instead of
/// sleeping.
fn pause_for(asked: Instant, cache: Duration, interest: Duration) -> Option<Duration> {
    let left = interest.checked_sub(asked.elapsed())?;
    if left.is_zero() {
        None
    } else {
        Some(cache.min(left))
    }
}

#[cfg(test)]
async fn scripted_answer(mut answers: watch::Receiver<Option<Answer>>) -> Answer {
    loop {
        if let Some(answer) = *answers.borrow_and_update() {
            return answer;
        }
        if answers.changed().await.is_err() {
            return Answer::Keep;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    use tokio_util::sync::CancellationToken;
    use vessel_proto::ProjectId;
    use vessel_proto::registry::DiffStat;
    use vessel_proto::workspace::WorkspaceRoot;
    use vessel_workspace::GitError;

    use super::{
        Answer, Book, CACHE, DiffStats, INTEREST, LIMIT, Slot, classify, fresh, lock, pause_for,
        permit, remember, retire, should_wait,
    };

    fn slot_at(asked: Instant) -> Slot {
        Slot {
            epoch: 0,
            roots: Vec::new(),
            seen: false,
            stat: None,
            asked,
        }
    }

    fn folder() -> (PathBuf, WorkspaceRoot) {
        (
            PathBuf::from("/no/such/vessel-diff-stat"),
            WorkspaceRoot::Project {
                project: ProjectId::random(),
            },
        )
    }

    #[test]
    fn publish_bumps_for_the_first_null_and_then_only_on_a_change() {
        let mut book = Book::default();
        let key = PathBuf::from("repo");
        let _previous = book.slots.insert(key.clone(), slot_at(Instant::now()));
        assert_eq!(book.publish(&key, None), Some(1));
        assert_eq!(book.publish(&key, None), None);
        let stat = DiffStat {
            additions: 1,
            deletions: 0,
        };
        assert_eq!(book.publish(&key, Some(stat)), Some(2));
        assert_eq!(book.publish(&key, Some(stat)), None);
        assert!(
            Book::default()
                .publish(Path::new("missing"), None)
                .is_none()
        );
        book.generation = u64::MAX;
        let next = DiffStat {
            additions: 2,
            deletions: 0,
        };
        assert!(book.publish(&key, Some(next)).is_none());
        assert_eq!(book.generation, u64::MAX);
        assert_eq!(book.slots[&key].stat, Some(next));
    }

    #[test]
    fn the_pause_is_the_cache_until_interest_runs_out() {
        let now = Instant::now();
        assert_eq!(pause_for(now, CACHE, INTEREST), Some(CACHE));
        let almost = now.checked_sub(Duration::from_secs(50)).unwrap();
        let left = pause_for(almost, CACHE, INTEREST).unwrap();
        assert!(left <= Duration::from_secs(10));
        assert!(left > Duration::from_secs(9));
        let done = now.checked_sub(INTEREST).unwrap();
        assert_eq!(pause_for(done, CACHE, INTEREST), None);
    }

    #[test]
    fn interest_removes_the_slot_and_a_newer_task_is_left_in_place() {
        let shared = fresh(CancellationToken::new(), CACHE, INTEREST);
        let key = PathBuf::from("repo");
        let old = Instant::now().checked_sub(INTEREST).unwrap();
        let _previous = lock(&shared.book).slots.insert(key.clone(), slot_at(old));
        assert!(retire(&shared, &key, 0));
        assert!(!lock(&shared.book).slots.contains_key(&key));

        let _previous = lock(&shared.book)
            .slots
            .insert(key.clone(), slot_at(Instant::now()));
        assert!(!retire(&shared, &key, 0));
        assert!(lock(&shared.book).slots.contains_key(&key));
        {
            let mut book = lock(&shared.book);
            let Some(slot) = book.slots.get_mut(&key) else {
                panic!("the slot was removed");
            };
            slot.epoch = 2;
        }
        assert!(retire(&shared, &key, 0));
        assert!(lock(&shared.book).slots.contains_key(&key));
    }

    #[test]
    fn a_busy_answer_keeps_the_stored_count_and_a_real_null_replaces_it() {
        let shared = fresh(CancellationToken::new(), CACHE, INTEREST);
        let key = PathBuf::from("repo");
        let _previous = lock(&shared.book)
            .slots
            .insert(key.clone(), slot_at(Instant::now()));
        let stat = DiffStat {
            additions: 1,
            deletions: 0,
        };
        remember(&shared, &key, Answer::Count(Some(stat)));
        let generation = lock(&shared.book).generation;
        remember(&shared, &key, Answer::Keep);
        assert_eq!(lock(&shared.book).slots[&key].stat, Some(stat));
        assert_eq!(lock(&shared.book).generation, generation);
        remember(&shared, &key, Answer::Count(None));
        assert!(lock(&shared.book).slots[&key].stat.is_none());
        assert!(lock(&shared.book).generation > generation);
    }

    #[test]
    fn a_busy_or_timed_out_count_is_kept_and_a_missing_base_is_null() {
        assert_eq!(classify(Ok(None)), Answer::Count(None));
        let stat = DiffStat {
            additions: 1,
            deletions: 0,
        };
        assert_eq!(classify(Ok(Some(stat))), Answer::Count(Some(stat)));
        assert_eq!(classify(Err(GitError::Busy)), Answer::Keep);
        assert_eq!(
            classify(Err(GitError::TimedOut {
                after: Duration::from_secs(1),
            })),
            Answer::Keep
        );
        assert_eq!(
            classify(Err(GitError::Unexpected { what: "base" })),
            Answer::Count(None)
        );
        assert_eq!(classify(Err(GitError::Closed)), Answer::Keep);
    }

    #[test]
    fn a_first_read_waits_until_counted_and_a_delegate_does_not() {
        assert!(should_wait(None, 0, true, false));
        assert!(!should_wait(None, 1, false, false));
        assert!(should_wait(Some(4), 4, false, false));
        assert!(!should_wait(Some(3), 4, false, false));
        assert!(!should_wait(None, 0, true, true));
        assert!(!should_wait(Some(4), 4, true, true));
    }

    #[tokio::test]
    async fn a_quiet_folder_is_dropped_after_interest() {
        let interest = Duration::from_millis(80);
        let (stats, tx) = DiffStats::scripted(Duration::from_millis(20), interest);
        tx.send(Some(Answer::Plain)).unwrap();
        let (path, root) = folder();
        stats.observe(&[path.clone()], &[(path.clone(), root)]);
        assert!(lock(&stats.shared.book).slots.contains_key(&path));
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline && lock(&stats.shared.book).slots.contains_key(&path) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(
            !lock(&stats.shared.book).slots.contains_key(&path),
            "the slot stays after interest"
        );
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline && !stats.tasks.is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(stats.tasks.is_empty());
    }

    #[tokio::test]
    async fn the_first_count_wakes_a_waiter_and_the_next_one_does_too() {
        let (stats, tx) = DiffStats::scripted(Duration::from_millis(40), Duration::from_secs(30));
        let (path, root) = folder();
        let mut changed = stats.subscribe();
        stats.observe(&[path.clone()], &[(path.clone(), root)]);
        assert!(!stats.ready(&path));
        assert!(should_wait(None, stats.generation(), true, false));

        let stat = DiffStat {
            additions: 2,
            deletions: 1,
        };
        tx.send(Some(Answer::Count(Some(stat)))).unwrap();
        tokio::time::timeout(Duration::from_secs(2), changed.changed())
            .await
            .unwrap()
            .unwrap();
        assert!(stats.ready(&path));
        assert_eq!(stats.stat(&path), Some(stat));
        let generation = stats.generation();
        assert_eq!(generation, 1);
        assert!(should_wait(Some(generation), generation, false, false));
        assert!(!should_wait(Some(generation), generation, false, true));

        let mut again = stats.subscribe();
        let next = DiffStat {
            additions: 4,
            deletions: 1,
        };
        tx.send(Some(Answer::Count(Some(next)))).unwrap();
        tokio::time::timeout(Duration::from_secs(2), again.changed())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stats.stat(&path), Some(next));
        assert!(stats.generation() > generation);
    }

    #[tokio::test]
    async fn the_loop_keeps_the_count_when_the_next_read_is_busy() {
        let (stats, tx) = DiffStats::scripted(Duration::from_millis(40), Duration::from_secs(30));
        let (path, root) = folder();
        let stat = DiffStat {
            additions: 1,
            deletions: 2,
        };
        tx.send(Some(Answer::Count(Some(stat)))).unwrap();
        stats.observe(&[path.clone()], &[(path.clone(), root)]);
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline && stats.stat(&path) != Some(stat) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(stats.stat(&path), Some(stat));
        let generation = stats.generation();
        tx.send(Some(Answer::Keep)).unwrap();
        tokio::time::sleep(Duration::from_millis(120)).await;
        assert_eq!(stats.stat(&path), Some(stat));
        assert_eq!(stats.generation(), generation);
        tx.send(Some(Answer::Count(None))).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline && stats.generation() == generation {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(stats.stat(&path).is_none());
        assert!(stats.ready(&path));
        assert!(stats.generation() > generation);
    }

    #[tokio::test]
    async fn at_most_four_counts_hold_a_slot() {
        let (stats, tx) = DiffStats::scripted(CACHE, INTEREST);
        drop(tx);
        let mut held = Vec::new();
        for _slot in 0..LIMIT {
            held.push(permit(&stats.shared).await.unwrap());
        }
        let shared = std::sync::Arc::clone(&stats.shared);
        let mut waiting = tokio::spawn(async move { permit(&shared).await });
        let blocked = tokio::time::timeout(Duration::from_millis(500), &mut waiting).await;
        assert!(blocked.is_err(), "a fifth count took a slot");
        drop(held);
        let acquired = tokio::time::timeout(Duration::from_secs(2), waiting)
            .await
            .unwrap()
            .unwrap();
        assert!(acquired.is_some());
    }
}
