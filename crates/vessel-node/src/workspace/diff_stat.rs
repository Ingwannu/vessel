//! Served line counts for `workspace.list` and `workspace.get` (ADR 0120).
//!
//! One watch per canonical folder, shared by the workspaces in it, dropped
//! [`INTEREST`] after the last request. Writes settle for [`DEBOUNCE`]. A
//! folder that is not a git repository, or whose watch did not start, is
//! counted again after [`CACHE`] and is not watched. At most [`MAX_SLOTS`]
//! folders are watched. Zero on both sides, and a failed read, are stored
//! as none. The generation moves only when a stored count changes,
//! including the first time it is stored.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use notify::{Event, RecursiveMode, Watcher};
use tokio::sync::{mpsc, watch};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use vessel_proto::registry::DiffStat;
use vessel_proto::workspace::WorkspaceRoot;

use super::Workspaces;

mod count;

/// How long a burst of writes settles before a new count.
const DEBOUNCE: Duration = Duration::from_secs(1);
/// How often a folder with no watch is counted again.
const CACHE: Duration = Duration::from_secs(15);
/// How long a folder stays watched after the last list or get.
const INTEREST: Duration = Duration::from_secs(60);
/// The most folders watched at once.
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
}

/// The node's served diff stats.
///
/// Watch tasks hold [`Workspaces`] and [`Shared`], not this value, so
/// dropping the service can cancel them.
#[derive(Debug)]
pub(crate) struct DiffStats {
    shared: Arc<Shared>,
    tasks: TaskTracker,
    workspaces: Arc<Workspaces>,
}

impl DiffStats {
    /// Watches start on the first request and stop with `workspaces`.
    #[must_use]
    pub(crate) fn start(workspaces: Arc<Workspaces>) -> Self {
        let stop = workspaces.stopping().child_token();
        let (changed, _receiver) = watch::channel(0);
        Self {
            shared: Arc::new(Shared {
                book: Mutex::new(Book::default()),
                changed,
                stop,
            }),
            tasks: TaskTracker::new(),
            workspaces,
        }
    }

    /// Remember interest in each folder and start a watch for one that is
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

    /// The stored count. None when it is null or the folder is not watched.
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
                book.slots.insert(
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
            let workspaces = Arc::clone(&self.workspaces);
            drop(self.tasks.spawn(async move {
                watch_cwd(key, epoch, shared, workspaces).await;
            }));
        }
    }
}

impl Drop for DiffStats {
    fn drop(&mut self) {
        self.shared.stop.cancel();
        self.tasks.close();
    }
}

fn lock(book: &Mutex<Book>) -> MutexGuard<'_, Book> {
    book.lock().unwrap_or_else(PoisonError::into_inner)
}

enum Folder {
    Git,
    Plain,
}

enum Wake {
    Stop,
    Quiet,
    Event,
    Closed,
}

struct Watch {
    watcher: Option<Box<notify::RecommendedWatcher>>,
    events: Option<mpsc::Receiver<()>>,
}

impl Watch {
    const fn empty() -> Self {
        Self {
            watcher: None,
            events: None,
        }
    }

    fn clear(&mut self) {
        self.watcher = None;
        self.events = None;
    }

    fn ensure(&mut self, dir: &Path) {
        if self.watcher.is_some() {
            return;
        }
        let Some((watcher, events)) = arm(dir) else {
            return;
        };
        self.watcher = Some(watcher);
        self.events = Some(events);
    }
}

async fn watch_cwd(key: PathBuf, epoch: u64, shared: Arc<Shared>, workspaces: Arc<Workspaces>) {
    let mut directory = Watch::empty();
    loop {
        if shared.stop.is_cancelled() || retire(&shared, &key, epoch) {
            break;
        }
        refresh(&workspaces, &shared, &key, &mut directory).await;
        if shared.stop.is_cancelled() || retire(&shared, &key, epoch) {
            break;
        }
        match wait(&shared.stop, &mut directory).await {
            Wake::Stop => break,
            Wake::Quiet | Wake::Closed => directory.clear(),
            Wake::Event => {
                if let Some(events) = directory.events.as_mut() {
                    settle(&shared.stop, events).await;
                }
            }
        }
    }
}

async fn refresh(workspaces: &Workspaces, shared: &Shared, key: &Path, directory: &mut Watch) {
    let (folder, stat) = match root_of(shared, key) {
        Some(root) => read_root(workspaces, root).await,
        None => (Folder::Plain, None),
    };
    remember(shared, key, stat);
    match folder {
        Folder::Git => directory.ensure(key),
        Folder::Plain => directory.clear(),
    }
}

async fn read_root(workspaces: &Workspaces, root: WorkspaceRoot) -> (Folder, Option<DiffStat>) {
    let resolved = match workspaces.resolve(root).await {
        Ok(resolved) if resolved.git => resolved,
        Ok(_) => return (Folder::Plain, None),
        Err(error) => {
            tracing::debug!(%error, "diff stat: the workspace folder is not available");
            return (Folder::Plain, None);
        }
    };
    let git = match workspaces.git() {
        Ok(git) => git,
        Err(error) => {
            tracing::debug!(%error, "diff stat: git is not available");
            return (Folder::Plain, None);
        }
    };
    let stat = count::stat_in(git, resolved.root.path(), resolved.base.as_ref()).await;
    (Folder::Git, stat)
}

fn root_of(shared: &Shared, key: &Path) -> Option<WorkspaceRoot> {
    lock(&shared.book)
        .slots
        .get(key)
        .and_then(|slot| slot.roots.last().copied())
}

fn remember(shared: &Shared, key: &Path, stat: Option<DiffStat>) {
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
    if slot.asked.elapsed() < INTEREST {
        return false;
    }
    book.slots.remove(key);
    true
}

async fn wait(stop: &CancellationToken, directory: &mut Watch) -> Wake {
    tokio::select! {
        () = stop.cancelled() => Wake::Stop,
        wake = listen(directory) => wake,
    }
}

async fn listen(directory: &mut Watch) -> Wake {
    match directory.events.as_mut() {
        Some(events) => match events.recv().await {
            Some(()) => Wake::Event,
            None => Wake::Closed,
        },
        None => {
            tokio::time::sleep(CACHE).await;
            Wake::Quiet
        }
    }
}

async fn settle(stop: &CancellationToken, events: &mut mpsc::Receiver<()>) {
    loop {
        tokio::select! {
            () = stop.cancelled() => return,
            () = tokio::time::sleep(DEBOUNCE) => return,
            received = events.recv() => {
                if received.is_none() {
                    return;
                }
            }
        }
    }
}

fn arm(dir: &Path) -> Option<(Box<notify::RecommendedWatcher>, mpsc::Receiver<()>)> {
    let (tx, rx) = mpsc::channel(1);
    let mut watcher =
        match notify::recommended_watcher(move |event: Result<Event, notify::Error>| {
            if event.as_ref().is_ok_and(ignored) {
                return;
            }
            drop(tx.try_send(()));
        }) {
            Ok(watcher) => watcher,
            Err(error) => {
                tracing::debug!(%error, "diff stat: the directory watch did not start");
                return None;
            }
        };
    if let Err(error) = watcher.watch(dir, RecursiveMode::Recursive) {
        tracing::debug!(%error, "diff stat: the directory watch did not start");
        return None;
    }
    Some((Box::new(watcher), rx))
}

fn ignored(event: &Event) -> bool {
    if event.kind.is_access() {
        return true;
    }
    !event.paths.is_empty() && event.paths.iter().map(PathBuf::as_path).all(skipped)
}

/// `node_modules`, `target`, and git's object and log stores.
fn skipped(path: &Path) -> bool {
    let mut under_git = false;
    for part in path.components() {
        let name = part.as_os_str();
        if name == "node_modules" || name == "target" {
            return true;
        }
        if under_git && (name == "objects" || name == "logs") {
            return true;
        }
        under_git = name == ".git";
    }
    false
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::Instant;

    use vessel_proto::registry::DiffStat;

    use super::{Book, Slot, skipped};

    #[test]
    fn publish_bumps_for_the_first_null_and_then_only_on_a_change() {
        let mut book = Book::default();
        let key = PathBuf::from("repo");
        book.slots.insert(
            key.clone(),
            Slot {
                epoch: 0,
                roots: Vec::new(),
                seen: false,
                stat: None,
                asked: Instant::now(),
            },
        );
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
        assert_eq!(book.slots.get(&key).unwrap().stat, Some(next));
    }

    #[test]
    fn dependency_and_git_object_paths_do_not_dirty_the_count() {
        assert!(skipped(Path::new("/repo/node_modules/pkg/index.js")));
        assert!(skipped(Path::new("/repo/target/debug/vessel")));
        assert!(skipped(Path::new("/repo/.git/objects/ab/cd")));
        assert!(skipped(Path::new("/repo/.git/logs/HEAD")));
        assert!(!skipped(Path::new("/repo/.git/HEAD")));
        assert!(!skipped(Path::new("/repo/src/main.rs")));
        assert!(!skipped(Path::new("/repo/target-not/file")));
        assert!(!skipped(Path::new("/repo/node_modules_backup/file")));
    }
}
