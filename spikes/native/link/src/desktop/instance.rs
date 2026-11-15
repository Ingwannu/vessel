//! User-local launch forwarding uses an OS lock and bounded local IPC, never TCP.

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};
use tokio::sync::{mpsc, oneshot};
use vessel_app_view::desktop::NavigationTarget;
use vessel_proto::{NodeId, SessionId};

use super::Error;

const QUEUE_CAPACITY: usize = 32;
const MAX_PACKET_BYTES: u32 = 4096;
const REQUEST_DEADLINE: std::time::Duration = std::time::Duration::from_secs(3);
const ACCEPTED: u8 = 1;
const RESTART_DEADLINE: std::time::Duration = std::time::Duration::from_secs(15);

#[derive(Clone, Copy)]
enum LockMode {
    Normal,
    Restart,
}

/// Parsed GUI launch intent, independent of argv or a transport encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    /// An ordinary second launch opens an additional window.
    NewWindow,
    /// Only the shared parser may construct a conversation destination.
    Session(NavigationTarget),
    /// An explicit absolute directory enters the project creation flow.
    Project(ProjectDirectory),
}

/// OS-local directory arguments cannot be relative to another process's cwd.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDirectory(PathBuf);

impl ProjectDirectory {
    /// Parse once before forwarding; existence is checked by the project flow.
    ///
    /// # Errors
    /// Relative paths and non-Unicode paths cannot cross this launch protocol.
    pub fn new(path: PathBuf) -> Result<Self, Error> {
        if !path.is_absolute() || path.to_str().is_none() {
            return Err(Error::InvalidLaunch);
        }
        Ok(Self(path))
    }
    /// The project flow remains the authority for registering this directory.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }
}

#[derive(Serialize, Deserialize)]
enum Packet {
    NewWindow,
    Session { node: NodeId, session: SessionId },
    Project(PathBuf),
}

impl From<&Launch> for Packet {
    fn from(launch: &Launch) -> Self {
        match launch {
            Launch::NewWindow => Self::NewWindow,
            Launch::Session(NavigationTarget::Session { node, session }) => Self::Session {
                node: *node,
                session: *session,
            },
            Launch::Project(directory) => Self::Project(directory.0.clone()),
        }
    }
}

impl TryFrom<Packet> for Launch {
    type Error = Error;
    fn try_from(packet: Packet) -> Result<Self, Self::Error> {
        match packet {
            Packet::NewWindow => Ok(Self::NewWindow),
            Packet::Session { node, session } => {
                Ok(Self::Session(NavigationTarget::Session { node, session }))
            }
            Packet::Project(path) => ProjectDirectory::new(path).map(Self::Project),
        }
    }
}

/// OS lock ownership is held until the local listener has stopped.
#[derive(Debug)]
pub struct Primary {
    stop: Option<oneshot::Sender<()>>,
    worker: Option<std::thread::JoinHandle<()>>,
    _lock: File,
}

impl Drop for Primary {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            drop(stop.send(()));
        }
        if let Some(worker) = self.worker.take() {
            drop(worker.join());
        }
    }
}

/// A primary receives launches; a successful secondary has already been acknowledged.
#[derive(Debug)]
pub enum Instance {
    /// Keep the guard alive for the whole application lifetime.
    Primary {
        guard: Primary,
        launches: mpsc::Receiver<Launch>,
    },
    /// Exit without initializing a renderer or starting a local node.
    Forwarded,
}

/// Resolve one primary per explicit configuration root before GUI initialization.
///
/// # Errors
/// Returns lock, IPC, version/packet and acknowledgement failures with no argv text.
pub fn claim(directory: &Path, launch: &Launch) -> Result<Instance, Error> {
    claim_with(directory, launch, LockMode::Normal)
}

/// A relaunch waits for the old primary instead of forwarding into a quitting app.
///
/// # Errors
/// The bounded startup wait returns an explicit restart deadline or OS error.
pub fn claim_restart(directory: &Path, launch: &Launch) -> Result<Instance, Error> {
    claim_with(directory, launch, LockMode::Restart)
}

struct LockWait {
    result: std::sync::mpsc::Receiver<Result<File, std::io::Error>>,
    waiting: std::sync::mpsc::Receiver<()>,
}

fn lock_waiter(file: File) -> Result<LockWait, Error> {
    let (done, result) = std::sync::mpsc::sync_channel(1);
    let (started, waiting) = std::sync::mpsc::sync_channel(1);
    let _worker = std::thread::Builder::new()
        .name("vessel-restart-lock".into())
        .spawn(move || {
            let acquired = match file.try_lock() {
                Ok(()) => Ok(file),
                Err(std::fs::TryLockError::WouldBlock) => {
                    drop(started.send(()));
                    file.lock().map(|()| file)
                }
                Err(std::fs::TryLockError::Error(error)) => Err(error),
            };
            drop(done.send(acquired));
        })?;
    Ok(LockWait { result, waiting })
}

fn wait_for_lock(file: File) -> Result<File, Error> {
    // OS acquisition blocks in the kernel. A bounded startup receiver avoids
    // polling; if the error-only app closes, process exit ends this helper.
    let LockWait { result, waiting } = lock_waiter(file)?;
    drop(waiting);
    result
        .recv_timeout(RESTART_DEADLINE)
        .map_err(|error| match error {
            std::sync::mpsc::RecvTimeoutError::Timeout => Error::RestartDeadline,
            std::sync::mpsc::RecvTimeoutError::Disconnected => Error::Closed,
        })?
        .map_err(Error::from)
}

fn claim_with(directory: &Path, launch: &Launch, mode: LockMode) -> Result<Instance, Error> {
    if !directory.is_absolute() {
        return Err(Error::RelativePath);
    }
    let directory = directory.join("desktop-instance");
    crate::prefs::private_dir(&directory)?;
    #[cfg(unix)]
    unix::check_directory(&directory)?;
    let mut lock = match crate::prefs::new_private_file(&directory.join("lock")) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(directory.join("lock"))?
        }
        Err(error) => return Err(error.into()),
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let acquired = match lock.try_lock() {
        Ok(()) => true,
        Err(std::fs::TryLockError::WouldBlock) => match mode {
            LockMode::Normal => false,
            LockMode::Restart => {
                lock = wait_for_lock(lock)?;
                true
            }
        },
        Err(std::fs::TryLockError::Error(error)) => return Err(error.into()),
    };
    if acquired {
        let (sender, launches) = mpsc::channel(QUEUE_CAPACITY);
        let (stop, cancel) = oneshot::channel();
        #[cfg(unix)]
        let listener = unix::bind(&directory, &runtime)?;
        #[cfg(windows)]
        let listener = windows::bind(&directory, &runtime)?;
        let worker = std::thread::Builder::new()
            .name("vessel-desktop-launch".into())
            .spawn(move || {
                #[cfg(unix)]
                runtime.block_on(unix::serve(listener, sender, cancel));
                #[cfg(windows)]
                runtime.block_on(windows::serve(listener, sender, cancel));
            })?;
        Ok(Instance::Primary {
            guard: Primary {
                stop: Some(stop),
                worker: Some(worker),
                _lock: lock,
            },
            launches,
        })
    } else {
        #[cfg(unix)]
        runtime.block_on(unix::forward(&directory, launch))?;
        #[cfg(windows)]
        runtime.block_on(windows::forward(&directory, launch))?;
        Ok(Instance::Forwarded)
    }
}

async fn receive(
    stream: &mut (impl AsyncRead + AsyncWrite + Unpin),
    sender: &mpsc::Sender<Launch>,
) -> Result<(), Error> {
    let length = stream.read_u32().await?;
    if length > MAX_PACKET_BYTES {
        return Err(Error::InvalidLaunch);
    }
    let length = usize::try_from(length).map_err(|_overflow| Error::InvalidLaunch)?;
    let mut bytes = vec![0; length];
    let _read = stream.read_exact(&mut bytes).await?;
    let packet: Packet = serde_json::from_slice(&bytes)?;
    sender
        .send(Launch::try_from(packet)?)
        .await
        .map_err(|_closed| Error::Closed)?;
    stream.write_u8(ACCEPTED).await?;
    Ok(())
}

async fn send(
    stream: &mut (impl AsyncRead + AsyncWrite + Unpin),
    launch: &Launch,
) -> Result<(), Error> {
    let bytes = serde_json::to_vec(&Packet::from(launch))?;
    let length = u32::try_from(bytes.len()).map_err(|_overflow| Error::InvalidLaunch)?;
    if length > MAX_PACKET_BYTES {
        return Err(Error::InvalidLaunch);
    }
    stream.write_u32(length).await?;
    stream.write_all(&bytes).await?;
    if stream.read_u8().await? != ACCEPTED {
        return Err(Error::InvalidLaunch);
    }
    Ok(())
}

async fn bounded<T>(future: impl Future<Output = Result<T, Error>>) -> Result<T, Error> {
    tokio::time::timeout(REQUEST_DEADLINE, future)
        .await
        .map_err(|_expired| Error::LaunchDeadline)?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_second_launch_forwards_and_relaunch_waits_for_the_old_guard() {
        let root = std::env::temp_dir().join(format!("vessel-instance-{}", SessionId::random()));
        let Instance::Primary {
            guard,
            mut launches,
        } = claim(&root, &Launch::NewWindow).unwrap()
        else {
            panic!("first launch must own IPC");
        };
        let target = Launch::Session(NavigationTarget::Session {
            node: NodeId::random(),
            session: SessionId::random(),
        });
        assert!(matches!(
            claim(&root, &target).unwrap(),
            Instance::Forwarded
        ));
        assert_eq!(launches.blocking_recv(), Some(target));
        let (ready, started) = std::sync::mpsc::sync_channel(1);
        let directory = root.clone();
        let restarted = std::thread::spawn(move || {
            ready.send(()).unwrap();
            claim_restart(&directory, &Launch::NewWindow).unwrap()
        });
        started.recv_timeout(REQUEST_DEADLINE).unwrap();
        drop(guard);
        let Instance::Primary {
            guard: new_guard,
            launches: _new_launches,
        } = restarted.join().unwrap()
        else {
            panic!("restart cannot forward");
        };
        drop(new_guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_shared_ipc_directory_is_refused_instead_of_opened_to_other_accounts() {
        use std::os::unix::fs::PermissionsExt as _;
        let root =
            std::env::temp_dir().join(format!("vessel-instance-private-{}", SessionId::random()));
        let directory = root.join("desktop-instance");
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(
            matches!(claim(&root,&Launch::NewWindow),Err(Error::Io(error)) if error.kind()==std::io::ErrorKind::PermissionDenied)
        );
        assert!(!directory.join("lock").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn packets_validate_identity_and_acknowledge_after_bounded_delivery() {
        let (mut client, mut server) = tokio::io::duplex(8192);
        let (sender, mut receiver) = mpsc::channel(1);
        let request = Launch::Session(NavigationTarget::Session {
            node: NodeId::random(),
            session: SessionId::random(),
        });
        let (sent, received) =
            tokio::join!(send(&mut client, &request), receive(&mut server, &sender));
        sent.unwrap();
        received.unwrap();
        assert_eq!(receiver.recv().await, Some(request));
        assert!(Launch::try_from(Packet::Project(PathBuf::from("relative"))).is_err());
    }

    #[tokio::test]
    async fn oversized_frame_is_refused_before_allocating_its_body() {
        let (mut client, mut server) = tokio::io::duplex(16);
        let (sender, _receiver) = mpsc::channel(1);
        client
            .write_u32(MAX_PACKET_BYTES.saturating_add(1))
            .await
            .unwrap();
        assert!(matches!(
            receive(&mut server, &sender).await,
            Err(Error::InvalidLaunch)
        ));
    }
}
