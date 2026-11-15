//! Finding files by name (`file.find`): what `@` in the composer and
//! open-by-name call on every keystroke, so it answers from memory.
//!
//! [Decision Log]
//! - Purpose: a search per keystroke on a repository of hundreds of
//!   thousands of files, answered in milliseconds.
//! - Alternatives: a walk per search (Paseo: at most 20,000 entries, a
//!   cache of 8 s); a native index library (GOAT's `fff`, 25,000 entries);
//!   a substring over the cached `git ls-files` list (agent-orchestrator).
//! - Choice: an index per root in memory, the list `git ls-files --cached
//!   --others --exclude-standard` gives (git's own ignore rules, read from
//!   its index, which is how a large repository answers fast) or a walk for
//!   a folder without git; at most [`MAX_INDEXED`] paths. Each path keeps a
//!   lower-cased copy, so a search lower-cases only the query. A search is
//!   a fuzzy subsequence match scored for word starts, runs and the file
//!   name, keeping the best `limit` in a heap, never sorting everything;
//!   it checks its cancellation every few thousand paths, so a newer
//!   keystroke's search stops the older one (the node keys them).
//! - [`Indexes`] keeps at most [`MAX_ROOTS`] of them, drops one unused for
//!   [`IDLE`], and rebuilds one older than [`STALE`] in the background
//!   while the old one keeps answering: a new file shows within seconds,
//!   and no keystroke waits for a rebuild after the first.
//! - Cost: memory, about twice the paths' length (the lower-cased copy);
//!   500,000 paths of 60 bytes is about 60 MB for one root at its bound.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use vessel_proto::workspace::{FoundFile, FoundFiles, RelPath};

use crate::git::Git;
use crate::paths::Root;

/// The most paths one index holds.
pub const MAX_INDEXED: usize = 500_000;
/// The most indexes kept.
pub const MAX_ROOTS: usize = 8;
/// An index unused this long is dropped.
pub const IDLE: Duration = Duration::from_secs(300);
/// An index older than this is rebuilt in the background on its next use.
pub const STALE: Duration = Duration::from_secs(10);
/// The most results a search returns.
pub const MAX_RESULTS: u32 = 200;
/// A search's results when not asked.
pub const DEFAULT_RESULTS: u32 = 50;
/// How many paths a search scores between looks at its cancellation.
const CHECK_EVERY: usize = 4096;

/// One root's paths.
#[derive(Debug)]
pub struct NameIndex {
    entries: Vec<Entry>,
    complete: bool,
    built: Instant,
}

#[derive(Debug)]
struct Entry {
    path: RelPath,
    lower: Box<str>,
    /// Where the file name starts, in bytes.
    name_start: usize,
    /// Whether the path is ASCII: then it is scored byte by byte, with no
    /// allocation (the common case, and the fast one).
    ascii: bool,
}

impl NameIndex {
    /// Build `root`'s index: git's list for a repository, a walk otherwise.
    pub async fn build(git: Option<&Git>, root: &Root) -> Self {
        let (paths, complete) = match git {
            Some(git) => match git.files(root.path()).await {
                Ok(found) => found,
                Err(error) => {
                    tracing::warn!(%error, "listing a repository's files failed; walking it");
                    walk(root.path().to_path_buf()).await
                }
            },
            None => walk(root.path().to_path_buf()).await,
        };
        Self::from_paths(paths, complete)
    }

    /// An index of `paths`.
    #[must_use]
    pub fn from_paths(paths: Vec<RelPath>, complete: bool) -> Self {
        let entries = paths
            .into_iter()
            .map(|path| {
                let lower = path.as_str().to_lowercase().into_boxed_str();
                Entry {
                    name_start: lower.rfind('/').map_or(0, |slash| slash.saturating_add(1)),
                    ascii: path.as_str().is_ascii(),
                    lower,
                    path,
                }
            })
            .collect();
        Self {
            entries,
            complete,
            built: Instant::now(),
        }
    }

    /// How many paths it holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether it holds none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The best `limit` paths for `query`, best first; `superseded` when
    /// `cancel` fired before it finished.
    #[must_use]
    pub fn search(
        &self,
        query: &str,
        limit: Option<u32>,
        cancel: &CancellationToken,
    ) -> FoundFiles {
        let limit =
            usize::try_from(limit.unwrap_or(DEFAULT_RESULTS).clamp(1, MAX_RESULTS)).unwrap_or(1);
        let needle: Vec<char> = query
            .to_lowercase()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        let ascii_needle: Option<Vec<u8>> = needle
            .iter()
            .all(char::is_ascii)
            .then(|| needle.iter().collect::<String>().into_bytes());
        let indexed = u32::try_from(self.entries.len()).unwrap_or(u32::MAX);
        let mut best: BinaryHeap<Reverse<(i64, Reverse<usize>, usize)>> = BinaryHeap::new();
        for (index, entry) in self.entries.iter().enumerate() {
            if index % CHECK_EVERY == 0 && cancel.is_cancelled() {
                return FoundFiles {
                    matches: Vec::new(),
                    indexed,
                    complete: self.complete,
                    superseded: true,
                };
            }
            let scored = if entry.ascii && ascii_needle.is_some() {
                ascii_needle.as_deref().and_then(|needle| {
                    score_ascii(
                        needle,
                        entry.lower.as_bytes(),
                        entry.path.as_str().as_bytes(),
                        entry.name_start,
                    )
                })
            } else {
                score(&needle, &entry.lower, entry.path.as_str())
            };
            let Some(score) = scored else {
                continue;
            };
            let key = Reverse((score, Reverse(entry.path.as_str().len()), index));
            if best.len() < limit {
                best.push(key);
            } else if best.peek().is_some_and(|worst| key < *worst) {
                let _dropped = best.pop();
                best.push(key);
            }
        }
        let mut ranked: Vec<_> = best.into_iter().map(|Reverse(key)| key).collect();
        ranked.sort_by(|a, b| b.cmp(a));
        let matches = ranked
            .into_iter()
            .filter_map(|(_, _, index)| self.entries.get(index))
            .map(|entry| FoundFile {
                positions: positions(&needle, &entry.lower),
                path: entry.path.clone(),
            })
            .collect();
        FoundFiles {
            matches,
            indexed,
            complete: self.complete,
            superseded: false,
        }
    }
}

/// How well `lower` (a path, lower-cased; `original` as it is) matches
/// `needle`, or `None` when its characters are not all there in order.
/// Higher is better: matches in the file name, at the start of a word, and
/// in runs score more; long paths and gaps score less.
pub(crate) fn score(needle: &[char], lower: &str, original: &str) -> Option<i64> {
    if needle.is_empty() {
        return Some(0_i64.saturating_sub(i64::try_from(lower.len()).unwrap_or(i64::MAX)));
    }
    let name_start = lower.rfind('/').map_or(0, |slash| slash.saturating_add(1));
    let in_name = lower
        .get(name_start..)
        .and_then(|name| align(needle, name).map(|found| (found, name_start)));
    let (found, offset, bonus) = match in_name {
        Some((found, offset)) => (found, offset, 60),
        None => (align(needle, lower)?, 0, 0),
    };
    let originals: Vec<char> = original.chars().collect();
    let lowers: Vec<char> = lower.chars().collect();
    let offset_chars = lower
        .get(..offset)
        .map_or(0, |prefix| prefix.chars().count());
    let mut total: i64 = bonus;
    let mut previous: Option<usize> = None;
    for position in found {
        let at = position.saturating_add(offset_chars);
        total = total.saturating_add(10);
        let before = at.checked_sub(1).and_then(|index| lowers.get(index));
        let boundary = match before {
            None => true,
            Some(c) => {
                matches!(c, '/' | '_' | '-' | '.' | ' ')
                    || (originals.get(at).is_some_and(|c| c.is_uppercase())
                        && at
                            .checked_sub(1)
                            .and_then(|index| originals.get(index))
                            .is_some_and(|c| c.is_lowercase()))
            }
        };
        if boundary {
            total = total.saturating_add(15);
        }
        if let Some(previous) = previous {
            if at == previous.saturating_add(1) {
                total = total.saturating_add(20);
            } else {
                let gap = i64::try_from(at.saturating_sub(previous)).unwrap_or(i64::MAX);
                total = total.saturating_sub(gap.min(10));
            }
        }
        previous = Some(at);
    }
    if lower.get(name_start..).is_some_and(|name| {
        let query: String = needle.iter().collect();
        name.starts_with(&query)
    }) {
        total = total.saturating_add(40);
    }
    let length = i64::try_from(lowers.len()).unwrap_or(i64::MAX);
    Some(total.saturating_sub(length.checked_div(4).unwrap_or(0)))
}

/// [`score`] for an ASCII path and query, byte by byte and without
/// allocating: the same rules, so both rank alike.
fn score_ascii(needle: &[u8], lower: &[u8], original: &[u8], name_start: usize) -> Option<i64> {
    if needle.is_empty() {
        return Some(0_i64.saturating_sub(i64::try_from(lower.len()).unwrap_or(i64::MAX)));
    }
    if !is_subsequence(needle, lower) {
        return None;
    }
    let mut total = match aligned_ascii(needle, lower, original, name_start) {
        Some(total) => total.saturating_add(60),
        None => aligned_ascii(needle, lower, original, 0)?,
    };
    if lower
        .get(name_start..)
        .is_some_and(|name| name.starts_with(needle))
    {
        total = total.saturating_add(40);
    }
    let length = i64::try_from(lower.len()).unwrap_or(i64::MAX);
    Some(total.saturating_sub(length.checked_div(4).unwrap_or(0)))
}

/// Whether `needle`'s bytes appear in `text` in order.
fn is_subsequence(needle: &[u8], text: &[u8]) -> bool {
    let mut wanted = needle.iter();
    let mut next = wanted.next();
    for byte in text {
        if next == Some(byte) {
            next = wanted.next();
            if next.is_none() {
                return true;
            }
        }
    }
    next.is_none()
}

/// The score of `needle` aligned leftmost in `lower` from `start`, or
/// `None` when it does not fit there.
fn aligned_ascii(needle: &[u8], lower: &[u8], original: &[u8], start: usize) -> Option<i64> {
    let mut total: i64 = 0;
    let mut previous: Option<usize> = None;
    let mut at = start;
    for want in needle {
        let found = lower
            .get(at..)?
            .iter()
            .position(|byte| byte == want)?
            .saturating_add(at);
        total = total.saturating_add(10);
        let boundary = match found.checked_sub(1) {
            None => true,
            Some(before) => {
                matches!(lower.get(before), Some(b'/' | b'_' | b'-' | b'.' | b' '))
                    || (original.get(found).is_some_and(u8::is_ascii_uppercase)
                        && original.get(before).is_some_and(u8::is_ascii_lowercase))
            }
        };
        if boundary {
            total = total.saturating_add(15);
        }
        if let Some(previous) = previous {
            if found == previous.saturating_add(1) {
                total = total.saturating_add(20);
            } else {
                let gap = i64::try_from(found.saturating_sub(previous)).unwrap_or(i64::MAX);
                total = total.saturating_sub(gap.min(10));
            }
        }
        previous = Some(found);
        at = found.saturating_add(1);
    }
    Some(total)
}

/// The positions (in characters of `text`) where `needle`'s characters
/// are found in order, leftmost; `None` when they are not all there.
fn align(needle: &[char], text: &str) -> Option<Vec<usize>> {
    let mut found = Vec::with_capacity(needle.len());
    let mut wanted = needle.iter();
    let mut next = wanted.next();
    for (index, c) in text.chars().enumerate() {
        match next {
            Some(want) if *want == c => {
                found.push(index);
                next = wanted.next();
            }
            Some(_) => {}
            None => break,
        }
    }
    next.is_none().then_some(found)
}

/// Where a result's characters matched, for highlighting.
fn positions(needle: &[char], lower: &str) -> Vec<u32> {
    if needle.is_empty() {
        return Vec::new();
    }
    let name_start = lower.rfind('/').map_or(0, |slash| slash.saturating_add(1));
    let offset = lower
        .get(..name_start)
        .map_or(0, |prefix| prefix.chars().count());
    let found = lower
        .get(name_start..)
        .and_then(|name| align(needle, name))
        .map(|found| {
            found
                .into_iter()
                .map(|at| at.saturating_add(offset))
                .collect()
        })
        .or_else(|| align(needle, lower))
        .unwrap_or_default();
    found
        .into_iter()
        .filter_map(|at| u32::try_from(at).ok())
        .collect()
}

/// A folder's files without git: every file and link below it, `.git`
/// folders skipped, links to folders not followed, at most
/// [`MAX_INDEXED`].
async fn walk(root: PathBuf) -> (Vec<RelPath>, bool) {
    tokio::task::spawn_blocking(move || {
        let mut paths = Vec::new();
        let mut folders = vec![RelPath::root()];
        while let Some(folder) = folders.pop() {
            let Ok(entries) = std::fs::read_dir(folder.under(&root)) else {
                continue;
            };
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                let Ok(path) = folder.child(&name) else {
                    continue;
                };
                let Ok(kind) = entry.file_type() else {
                    continue;
                };
                if kind.is_dir() {
                    folders.push(path);
                } else {
                    if paths.len() >= MAX_INDEXED {
                        return (paths, false);
                    }
                    paths.push(path);
                }
            }
        }
        (paths, true)
    })
    .await
    .unwrap_or_default()
}

/// The indexes of the roots in use, rebuilt when stale.
#[derive(Debug, Default)]
pub struct Indexes {
    slots: Mutex<HashMap<PathBuf, Arc<Slot>>>,
}

#[derive(Debug)]
struct Slot {
    index: RwLock<Option<Arc<NameIndex>>>,
    used: Mutex<Instant>,
    refreshing: AtomicBool,
}

impl Indexes {
    /// `root`'s index: the one kept (rebuilt in the background when stale),
    /// or a new one.
    pub async fn get(self: &Arc<Self>, git: Option<&Git>, root: &Root) -> Arc<NameIndex> {
        let slot = self.slot(root.path());
        if let Some(index) = slot.index.read().await.clone() {
            if index.built.elapsed() > STALE && !slot.refreshing.swap(true, Ordering::AcqRel) {
                let (git, root, slot) = (git.cloned(), root.clone(), Arc::clone(&slot));
                drop(tokio::spawn(async move {
                    let fresh = Arc::new(NameIndex::build(git.as_ref(), &root).await);
                    *slot.index.write().await = Some(fresh);
                    slot.refreshing.store(false, Ordering::Release);
                }));
            }
            return index;
        }
        // The first search of a root builds it; others asking meanwhile
        // wait on the same lock and take what it built.
        let mut guard = slot.index.write().await;
        if let Some(index) = guard.clone() {
            return index;
        }
        let index = Arc::new(NameIndex::build(git, root).await);
        *guard = Some(Arc::clone(&index));
        index
    }

    /// The slot for `path`, made when missing; the least recently used one
    /// and any idle one dropped past the bound.
    fn slot(&self, path: &Path) -> Arc<Slot> {
        let mut slots = self.slots.lock().unwrap_or_else(PoisonError::into_inner);
        let now = Instant::now();
        slots.retain(|_, slot| {
            now.duration_since(*slot.used.lock().unwrap_or_else(PoisonError::into_inner)) < IDLE
        });
        if let Some(slot) = slots.get(path) {
            *slot.used.lock().unwrap_or_else(PoisonError::into_inner) = now;
            return Arc::clone(slot);
        }
        if slots.len() >= MAX_ROOTS {
            let oldest = slots
                .iter()
                .min_by_key(|(_, slot)| *slot.used.lock().unwrap_or_else(PoisonError::into_inner))
                .map(|(path, _)| path.clone());
            if let Some(oldest) = oldest {
                drop(slots.remove(&oldest));
            }
        }
        let slot = Arc::new(Slot {
            index: RwLock::new(None),
            used: Mutex::new(now),
            refreshing: AtomicBool::new(false),
        });
        drop(slots.insert(path.to_path_buf(), Arc::clone(&slot)));
        slot
    }
}

#[cfg(test)]
mod tests;
