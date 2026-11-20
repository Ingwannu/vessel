//! Bounded root/language actors. A server owns all mutable LSP state.
//!
//! Servers are keyed by the workspace they analyse and accounted to the
//! allowed root the node granted, so slots are shared fairly between roots
//! (see `Pool`). Each server admits one active query and eight
//! queued queries; overload is refused. The manager never waits for a query
//! while routing another server. Idle and backoff timers are deadlines, not
//! filesystem/readiness polling.

use std::collections::{BTreeMap, VecDeque};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use vessel_proto::Reply;

use crate::error::LspError as Error;
use crate::query::Query;
use crate::registry::Registry;
use crate::toolchain::RustTools;

mod server;

/// Resource limits and trusted server selection, configured only by the node.
#[derive(Debug, Clone)]
pub struct Config {
    /// Trusted executables, empty by default so library tests never find real tools.
    pub registry: Registry,
    /// Concurrent servers, including those starting (default four).
    pub max_servers: NonZeroUsize,
    /// Time without queries before shutdown (default five minutes).
    pub idle_timeout: Duration,
    /// Whole-query and initialization deadline (default thirty seconds).
    pub request_timeout: Duration,
    /// First crash/startup retry delay, doubling to at most thirty seconds.
    pub restart_backoff: Duration,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            registry: Registry::default(),
            max_servers: NonZeroUsize::MIN.saturating_add(3),
            idle_timeout: Duration::from_secs(300),
            request_timeout: Duration::from_secs(30),
            restart_backoff: Duration::from_secs(1),
        }
    }
}

/// Where one query runs, as authorized by the node.
///
/// A server analyses `workspace`, the canonical directory the caller asked
/// for. `root` is the allowed root the node granted for it (a configured
/// root or a local session's cwd): server slots are shared fairly between
/// allowed roots, so many subdirectories of one root cannot lock another
/// root out.
#[derive(Debug, Clone)]
pub struct Scope {
    root: PathBuf,
    workspace: PathBuf,
    full_analysis: bool,
}

impl Scope {
    /// Both paths canonical; `workspace` at or below `root`.
    /// `full_analysis` is the operator's choice for `root`, never a caller's.
    ///
    /// # Errors
    /// [`Error::Path`] when `workspace` is outside `root`.
    pub fn new(root: PathBuf, workspace: PathBuf, full_analysis: bool) -> Result<Self, Error> {
        if !workspace.starts_with(&root) {
            return Err(Error::Path);
        }
        Ok(Self {
            root,
            workspace,
            full_analysis,
        })
    }
}

/// A server's identity: what it analyses, never who asked.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    workspace: PathBuf,
    language: Language,
    full: bool,
}
#[derive(Debug)]
struct Work {
    query: Query,
    reply: oneshot::Sender<Result<Reply, Error>>,
}
#[derive(Debug)]
struct Admission {
    key: Key,
    /// The allowed root this admission's slot is accounted to.
    root: PathBuf,
    work: Work,
}

/// The node's Rust toolchain, resolved at the first Rust launch and then
/// shared by every server of this manager (see `toolchain`).
type Tools = Arc<tokio::sync::OnceCell<RustTools>>;

/// Cheap sender to the node's LSP manager. Authority is checked by the node.
#[derive(Debug, Clone)]
pub struct Manager {
    sender: mpsc::Sender<Admission>,
    finished: tokio::sync::watch::Receiver<bool>,
}

impl Manager {
    /// Start a manager under the node's cancellation tree.
    #[must_use]
    pub fn start(config: Config, shutdown: CancellationToken) -> Self {
        let (sender, receiver) = mpsc::channel(32);
        let (done, finished) = tokio::sync::watch::channel(false);
        drop(tokio::spawn(async move {
            Pool::new(config, shutdown).run(receiver).await;
            let _old = done.send_replace(true);
        }));
        Self { sender, finished }
    }

    /// Wait until node cancellation has shut down every language-server group.
    pub async fn stopped(&self) {
        let mut finished = self.finished.clone();
        while !*finished.borrow_and_update() {
            if finished.changed().await.is_err() {
                break;
            }
        }
    }

    /// Query within an already-authorized scope.
    ///
    /// # Errors
    /// Refuses overload, shutdown, unavailable servers and invalid source data.
    pub async fn query(&self, scope: Scope, query: Query) -> Result<Reply, Error> {
        if let Some(file) = query.file() {
            let _canonical = crate::documents::resolve(&scope.workspace, file).await?;
        }
        let (reply, response) = oneshot::channel();
        let key = Key {
            workspace: scope.workspace,
            language: query.workspace().language,
            full: scope.full_analysis,
        };
        self.sender
            .try_send(Admission {
                key,
                root: scope.root,
                work: Work { query, reply },
            })
            .map_err(|_full_or_closed| Error::Unavailable)?;
        response.await.map_err(|_closed| Error::Unavailable)?
    }
}

/// Admissions waiting for a slot a retiring server is about to free.
const MAX_PARKED: usize = 8;

#[derive(Debug)]
struct Slot {
    sender: mpsc::Sender<Work>,
    /// The allowed root this server is accounted to (its first requester's).
    root: PathBuf,
    /// Asks the server to stop between queries, keeping any in-flight one.
    retire: CancellationToken,
    /// Set once the slot is being freed for a parked admission. From then
    /// on the slot counts toward that admission's root, not its own.
    retiring: Option<Replacement>,
    /// Admission order of the last query, for least-recently-used choice.
    used: u64,
}

/// Who a retiring slot is reserved for.
#[derive(Debug)]
struct Replacement {
    root: PathBuf,
    key: Key,
}

impl Slot {
    /// The root this slot counts toward in the fairness tally.
    fn holder(&self) -> &Path {
        self.retiring
            .as_ref()
            .map_or(self.root.as_path(), |replacement| {
                replacement.root.as_path()
            })
    }
}

/// [Decision Log]
/// - Purpose: a bounded number of servers, shared fairly between roots.
/// - Before: four subdirectories of one allowed root took every slot and
///   every other root was refused until they idled out (review finding).
/// - Alternatives: one server per allowed root, rooted there (a session cwd
///   such as a home directory would make pyright/gopls index the whole tree
///   and put user-installed tools "inside the root"); a fixed per-root quota
///   (wastes slots while one root is alone); refusing and asking callers to
///   retry (callers would have to poll for a slot).
/// - Chosen: servers keep the requested workspace as their analysis root
///   and are accounted to the allowed root. When all slots are taken, the
///   busiest root that holds more servers than the requesting root loses its
///   least recently used server; otherwise the requesting root recycles its
///   own. The admission waits (parked, bounded) for that slot, and the
///   retiring slot is reserved for it: it counts toward the waiting root, so
///   a burst from one root cannot see itself as empty and take a second
///   root's slot, and the freed slot goes to the admission it was freed for.
///   Parking room is reserved before any server is retired.
/// - Trade-off: retirement waits for an in-flight query; queries already
///   queued on the retired server are answered `Unavailable`. Admissions
///   parked without a reservation are retried, in arrival order, whenever a
///   server ends.
struct Pool {
    config: Config,
    shutdown: CancellationToken,
    servers: BTreeMap<Key, Slot>,
    parked: VecDeque<Admission>,
    /// Keys whose reserved slot has just been freed; served first.
    freed: Vec<Key>,
    tasks: JoinSet<()>,
    tools: Tools,
    clock: u64,
}

impl Pool {
    fn new(config: Config, shutdown: CancellationToken) -> Self {
        Self {
            config,
            shutdown,
            servers: BTreeMap::new(),
            parked: VecDeque::new(),
            freed: Vec::new(),
            tasks: JoinSet::new(),
            tools: Tools::default(),
            clock: 0,
        }
    }

    async fn run(mut self, mut inbox: mpsc::Receiver<Admission>) {
        loop {
            tokio::select! {
                () = self.shutdown.cancelled() => break,
                _ended = self.tasks.join_next(), if !self.tasks.is_empty() => self.unpark(),
                admission = inbox.recv() => {
                    let Some(admission) = admission else { break; };
                    self.admit(admission);
                }
            }
        }
        self.shutdown.cancel();
        drop(self.servers);
        drop(self.parked);
        while self.tasks.join_next().await.is_some() {}
    }

    /// Forget ended servers, remembering whose reserved slot they freed.
    fn prune(&mut self) {
        let freed = &mut self.freed;
        self.servers.retain(|_, slot| {
            let open = !slot.sender.is_closed();
            if !open && let Some(replacement) = &slot.retiring {
                freed.push(replacement.key.clone());
            }
            open
        });
    }

    /// A server ended: retry parked admissions once. Those a freed slot was
    /// reserved for go first; the rest keep their arrival order.
    fn unpark(&mut self) {
        self.prune();
        let freed = std::mem::take(&mut self.freed);
        let (first, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut self.parked)
            .into_iter()
            .partition(|admission| freed.contains(&admission.key));
        for admission in first.into_iter().chain(rest) {
            if !admission.work.reply.is_closed() {
                self.admit(admission);
            }
        }
    }

    fn admit(&mut self, admission: Admission) {
        self.prune();
        self.clock = self.clock.saturating_add(1);
        let Admission { key, root, work } = admission;
        if let Some(slot) = self.servers.get_mut(&key) {
            if slot.retiring.is_some() {
                self.park(Admission { key, root, work });
            } else {
                slot.used = self.clock;
                if let Err(error) = slot.sender.try_send(work) {
                    drop(error.into_inner().reply.send(Err(Error::Unavailable)));
                }
            }
            return;
        }
        if self.servers.len() < self.config.max_servers.get() {
            self.spawn(key, root, work);
            return;
        }
        let admission = Admission { key, root, work };
        // Room to wait comes first: never retire a server for an admission
        // that would then be refused.
        if self.parked.len() >= MAX_PARKED {
            drop(admission.work.reply.send(Err(Error::Unavailable)));
            return;
        }
        let reserved = self.servers.values().any(|slot| {
            slot.retiring
                .as_ref()
                .is_some_and(|replacement| replacement.key == admission.key)
        });
        if !reserved && let Some(victim) = self.victim(&admission.root) {
            if let Some(slot) = self.servers.get_mut(&victim) {
                slot.retiring = Some(Replacement {
                    root: admission.root.clone(),
                    key: admission.key.clone(),
                });
                slot.retire.cancel();
            }
            self.parked.push_back(admission);
        } else if self.servers.values().any(|slot| slot.retiring.is_some()) {
            self.parked.push_back(admission);
        } else {
            drop(admission.work.reply.send(Err(Error::Unavailable)));
        }
    }

    fn park(&mut self, admission: Admission) {
        if self.parked.len() >= MAX_PARKED {
            drop(admission.work.reply.send(Err(Error::Unavailable)));
        } else {
            self.parked.push_back(admission);
        }
    }

    /// The server to retire so `root` gets a slot: among roots holding more
    /// than `root`, the busiest one's least recently used server; else the
    /// least recently used of `root`'s own. A retiring slot counts toward
    /// the root it is reserved for and is never chosen again.
    fn victim(&self, root: &Path) -> Option<Key> {
        let mut held: BTreeMap<&Path, usize> = BTreeMap::new();
        for slot in self.servers.values() {
            let count = held.entry(slot.holder()).or_default();
            *count = count.saturating_add(1);
        }
        let tally = |owner: &Path| held.get(owner).copied().unwrap_or(0);
        let mine = tally(root);
        let live = self
            .servers
            .iter()
            .filter(|(_, slot)| slot.retiring.is_none());
        let busier = live
            .clone()
            .filter(|(_, slot)| tally(&slot.root) > mine)
            .max_by(|(_, left), (_, right)| {
                tally(&left.root)
                    .cmp(&tally(&right.root))
                    .then(right.used.cmp(&left.used))
            });
        busier
            .or_else(|| {
                live.filter(|(_, slot)| slot.root == root)
                    .min_by_key(|(_, slot)| slot.used)
            })
            .map(|(key, _)| key.clone())
    }

    fn spawn(&mut self, key: Key, root: PathBuf, work: Work) {
        let (sender, receiver) = mpsc::channel(8);
        let retire = CancellationToken::new();
        let task = server::run(
            key.clone(),
            self.config.clone(),
            self.shutdown.child_token(),
            retire.clone(),
            receiver,
            Arc::clone(&self.tools),
        );
        drop(self.tasks.spawn(task));
        if let Err(error) = sender.try_send(work) {
            drop(error.into_inner().reply.send(Err(Error::Unavailable)));
        }
        drop(self.servers.insert(
            key,
            Slot {
                sender,
                root,
                retire,
                retiring: None,
                used: self.clock,
            },
        ));
    }
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
