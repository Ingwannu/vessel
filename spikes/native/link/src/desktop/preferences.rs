//! One canonical preference actor applies typed deltas and broadcasts committed snapshots.

mod merge;

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::{Arc,mpsc};
use tokio::sync::{oneshot,watch};
use vessel_app_view::prefs::Kept;
use crate::Prefs;
use super::Error;

const QUEUE_CAPACITY:usize=64;

/// Revisions order outcomes as well as successful saves, so stale failures cannot win.
#[derive(Debug,Clone,Copy,Default,PartialEq,Eq,PartialOrd,Ord)]
pub struct Revision(u64);

/// Launch-only choices remain useful without pretending that a file was written.
#[derive(Debug,Clone)]
pub enum Outcome {
    /// The existing preferences file was committed.
    Saved,
    /// The initial file was unreadable; only memory was changed.
    LaunchOnly,
    /// The canonical snapshot is unchanged after a refused edit.
    Failed(Arc<Error>),
}

/// The file and every window observe the same canonical successful snapshot.
#[derive(Debug,Clone)]
pub struct Snapshot {
    revision:Revision,
    prefs:Arc<Prefs>,
    outcome:Outcome,
}

impl Snapshot {
    /// Revision includes failures and is monotonically ordered.
    #[must_use]
    pub const fn revision(&self) -> Revision {self.revision}
    /// Clone the Arc rather than every map when fanning out to windows.
    #[must_use]
    pub fn preferences(&self) -> Arc<Prefs> {Arc::clone(&self.prefs)}
    /// Each window can translate a failure without receiving file contents.
    #[must_use]
    pub const fn outcome(&self) -> &Outcome {&self.outcome}
}

enum Command {
    Change {before:Arc<Prefs>,after:Arc<Prefs>},
    Flush(oneshot::Sender<()>),
}

/// Process-wide handle; no per-window writer can race the atomic file helper.
#[derive(Debug)]
pub struct Coordinator {
    commands:mpsc::SyncSender<Command>,
    current:watch::Receiver<Snapshot>,
}

/// UI-thread client tracks its own optimistic baseline between acknowledgements.
#[derive(Debug)]
pub struct Client {
    commands:mpsc::SyncSender<Command>,
    baseline:RefCell<Arc<Prefs>>,
    revision:std::cell::Cell<Revision>,
}

impl Coordinator {
    /// Preserve the loaded path and launch-only refusal exactly as Prefs read them.
    ///
    /// # Errors
    /// Refuses a relative persisted path or a worker that cannot start.
    pub fn start(initial:Prefs) -> Result<Self,Error> {
        let path=match &initial.kept {
            Kept::File {path}=>{
                let path=PathBuf::from(path);
                if !path.is_absolute() {return Err(Error::RelativePath);}
                Some(path)
            }
            Kept::ThisLaunch {..}=>None,
        };
        let outcome=if path.is_some() {Outcome::Saved} else {Outcome::LaunchOnly};
        let snapshot=Snapshot {revision:Revision::default(),prefs:Arc::new(initial),outcome};
        let (published,current)=watch::channel(snapshot.clone());
        let (commands,receiver)=mpsc::sync_channel(QUEUE_CAPACITY);
        let _worker=std::thread::Builder::new().name("vessel-preferences".into())
            .spawn(move ||run(snapshot,path,published,receiver))?;
        Ok(Self {commands,current})
    }

    /// A new window starts from the latest committed snapshot, including launch-only edits.
    #[must_use]
    pub fn client(&self) -> (Client,watch::Receiver<Snapshot>) {
        let snapshot=self.current.borrow().clone();
        (Client {commands:self.commands.clone(),baseline:RefCell::new(snapshot.preferences()),revision:std::cell::Cell::new(snapshot.revision())},self.current.clone())
    }

    /// Window creation reads memory, not an older disk snapshot.
    #[must_use]
    pub fn current(&self) -> Arc<Prefs> {self.current.borrow().preferences()}

    /// Wait for every accepted edit before the process exits.
    ///
    /// # Errors
    /// Returns worker startup or closure failures; save errors remain in outcomes.
    pub fn flush(&self) -> impl Future<Output=Result<(),Error>>+'static {
        let commands=self.commands.clone();
        async move {
            let (done,wait)=oneshot::channel();
            let _sender=std::thread::Builder::new().name("vessel-preferences-flush".into())
                .spawn(move ||drop(commands.send(Command::Flush(done))))?;
            wait.await.map_err(|_closed|Error::Closed)
        }
    }
}

impl Client {
    /// Publish only changes since this window's last observed/submitted snapshot.
    ///
    /// # Errors
    /// A full or closed queue leaves the baseline unchanged for an explicit retry.
    pub fn publish(&self,prefs:Prefs) -> Result<(),Error> {
        let after=Arc::new(prefs);
        let before=Arc::clone(&self.baseline.borrow());
        self.commands.try_send(Command::Change {before,after:Arc::clone(&after)}).map_err(|error|match error {
            mpsc::TrySendError::Full(_)=>Error::Busy,mpsc::TrySendError::Disconnected(_)=>Error::Closed,
        })?;
        let _previous=self.baseline.replace(after);
        Ok(())
    }

    /// An older scheduled acknowledgement cannot replace a newer canonical revision.
    pub fn observe(&self,snapshot:&Snapshot) {
        if snapshot.revision>=self.revision.get() {
            self.revision.set(snapshot.revision);
            let _previous=self.baseline.replace(snapshot.preferences());
        }
    }
}

fn run(mut snapshot:Snapshot,path:Option<PathBuf>,published:watch::Sender<Snapshot>,receiver:mpsc::Receiver<Command>) {
    while let Ok(command)=receiver.recv() {
        match command {
            Command::Flush(done)=>drop(done.send(())),
            Command::Change {before,after}=>{
                let next=merge::apply(&snapshot.prefs,&before,&after).map_err(Error::from).and_then(|next| {
                    if let Some(path)=&path {next.save_at(path).map_err(Error::PreferencesWrite)?;}
                    Ok(next)
                });
                snapshot.revision=Revision(snapshot.revision.0.saturating_add(1));
                snapshot.outcome=match next {
                    Ok(next)=>{snapshot.prefs=Arc::new(next);if path.is_some() {Outcome::Saved} else {Outcome::LaunchOnly}}
                    Err(error)=>Outcome::Failed(Arc::new(error)),
                };
                published.send_replace(snapshot.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vessel_app_view::prefs::{Grouping,Theme};

    #[tokio::test]
    async fn stale_windows_merge_and_failed_save_keeps_canonical_state() {
        let root=std::env::temp_dir().join(format!("vessel-central-prefs-{}",std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let mut initial=Prefs::default();
        initial.kept=Kept::File {path:root.join("prefs.json").display().to_string()};
        let coordinator=Coordinator::start(initial.clone()).unwrap();
        let (first,_updates)=coordinator.client();
        let (second,mut updates)=coordinator.client();
        let mut one=initial.clone();one.settings.appearance.theme=Theme::Light;
        first.publish(one).unwrap();coordinator.flush().await.unwrap();
        let mut two=initial;two.settings.sidebar.group=Grouping::Status;
        second.publish(two).unwrap();coordinator.flush().await.unwrap();
        updates.changed().await.unwrap();
        assert_eq!(coordinator.current().settings.appearance.theme,Theme::Light);
        assert_eq!(coordinator.current().settings.sidebar.group,Grouping::Status);
        std::fs::remove_file(root.join("prefs.json")).unwrap();
        std::fs::create_dir(root.join("prefs.json")).unwrap();
        let committed=coordinator.current();
        let mut refused=(*committed).clone();refused.manual_updates=true;
        second.publish(refused).unwrap();coordinator.flush().await.unwrap();
        assert_eq!(coordinator.current(),committed);
        assert!(matches!(updates.borrow().outcome(),Outcome::Failed(_)));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn corrupt_file_remains_launch_only_and_never_gets_replaced() {
        let initial=Prefs::default();
        let coordinator=Coordinator::start(initial.clone()).unwrap();
        let (client,updates)=coordinator.client();
        let mut next=initial;next.manual_updates=true;
        client.publish(next).unwrap();coordinator.flush().await.unwrap();
        assert!(coordinator.current().manual_updates);
        assert!(matches!(updates.borrow().outcome(),Outcome::LaunchOnly));
    }
}
