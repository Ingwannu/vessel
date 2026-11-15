//! A single actor persists registry candidates before publishing them, even if a caller leaves.
//! Connectivity is supplied by existing paired/fleet authorities, never created here.

mod state;
use state::State;
use tokio::sync::{mpsc, oneshot};
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_proto::host_registry::{
    AddHostRegistration, HostDestination, HostRegistration, HostSource, RemoveHostRegistration,
    SharedHostRegistrations, UpdateHostRegistration,
};
use vessel_store::{
    DataDir,
    host_registry::{HostRegistryFile, RegistryFileError},
};

const COMMAND_QUEUE: usize = 32;
const RESPONSE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(30);

/// Pending RPC handlers will construct this solely from the existing trust authorities.
#[derive(Debug, Clone)]
pub(crate) struct TrustedDestinations(pub(crate) Vec<HostDestination>);
#[derive(Debug, thiserror::Error)]
pub(crate) enum RegistryError {
    #[error("host registry is busy or stopped")]
    Busy,
    #[error("host registry response exceeded its deadline")]
    Timeout(#[source] tokio::time::error::Elapsed),
    #[error("host registry worker failed")]
    Worker(#[source] tokio::task::JoinError),
    #[error(transparent)]
    File(#[from] RegistryFileError),
    #[error("host registration not found")]
    NotFound,
    #[error("host registration belongs to another authenticated source")]
    Forbidden,
    #[error("host destination is not an existing trusted paired/fleet destination")]
    Untrusted,
    #[error("host registration changed concurrently")]
    Stale,
    #[error("destination identity changed; explicit replacement is required")]
    IdentityChanged,
    #[error("host registry is full")]
    Full,
    #[error(transparent)]
    SharedInventory(#[from] vessel_proto::host_registry::TooManySharedHosts),
    #[error("host registration revision is exhausted")]
    RevisionExhausted,
}
#[derive(Debug, Clone)]
pub(crate) struct HostRegistry {
    commands: mpsc::Sender<Command>,
}
#[derive(Debug)]
enum Change {
    Add(AddHostRegistration, TrustedDestinations),
    Update(UpdateHostRegistration, TrustedDestinations),
    Remove(RemoveHostRegistration),
}
#[derive(Debug)]
enum Command {
    ListShared {
        reply: oneshot::Sender<Result<SharedHostRegistrations, RegistryError>>,
    },
    List {
        source: HostSource,
        reply: oneshot::Sender<Vec<HostRegistration>>,
    },
    Change {
        source: HostSource,
        change: Change,
        reply: oneshot::Sender<Result<Option<HostRegistration>, RegistryError>>,
    },
}
impl HostRegistry {
    /// Public projection happens inside the actor: no private records cross this boundary.
    pub(crate) async fn shared(&self) -> Result<SharedHostRegistrations, RegistryError> {
        let (reply, response) = oneshot::channel();
        self.commands
            .try_send(Command::ListShared { reply })
            .map_err(|_failure| RegistryError::Busy)?;
        tokio::time::timeout(RESPONSE_DEADLINE, response)
            .await
            .map_err(RegistryError::Timeout)?
            .map_err(|_failure| RegistryError::Busy)?
    }
    pub(crate) async fn start(
        dir: DataDir,
        stop: CancellationToken,
        tasks: &TaskTracker,
    ) -> Result<Self, RegistryError> {
        let read_dir = dir.clone();
        let file = tokio::task::spawn_blocking(move || read_dir.read_host_registry())
            .await
            .map_err(RegistryError::Worker)??;
        let (commands, receiver) = mpsc::channel(COMMAND_QUEUE);
        drop(
            tasks.spawn(
                Owner {
                    dir,
                    state: State::new(file),
                }
                .run(receiver, stop),
            ),
        );
        Ok(Self { commands })
    }
    pub(crate) async fn list(
        &self,
        source: HostSource,
    ) -> Result<Vec<HostRegistration>, RegistryError> {
        let (reply, response) = oneshot::channel();
        self.commands
            .try_send(Command::List { source, reply })
            .map_err(|_failure| RegistryError::Busy)?;
        tokio::time::timeout(RESPONSE_DEADLINE, response)
            .await
            .map_err(RegistryError::Timeout)?
            .map_err(|_failure| RegistryError::Busy)
    }
    pub(crate) async fn add(
        &self,
        source: HostSource,
        input: AddHostRegistration,
        trusted: TrustedDestinations,
    ) -> Result<HostRegistration, RegistryError> {
        self.change(source, Change::Add(input, trusted))
            .await?
            .ok_or(RegistryError::NotFound)
    }
    pub(crate) async fn update(
        &self,
        source: HostSource,
        input: UpdateHostRegistration,
        trusted: TrustedDestinations,
    ) -> Result<HostRegistration, RegistryError> {
        self.change(source, Change::Update(input, trusted))
            .await?
            .ok_or(RegistryError::NotFound)
    }
    pub(crate) async fn remove(
        &self,
        source: HostSource,
        input: RemoveHostRegistration,
    ) -> Result<(), RegistryError> {
        self.change(source, Change::Remove(input)).await.map(drop)
    }
    async fn change(
        &self,
        source: HostSource,
        change: Change,
    ) -> Result<Option<HostRegistration>, RegistryError> {
        let (reply, response) = oneshot::channel();
        self.commands
            .try_send(Command::Change {
                source,
                change,
                reply,
            })
            .map_err(|_failure| RegistryError::Busy)?;
        tokio::time::timeout(RESPONSE_DEADLINE, response)
            .await
            .map_err(RegistryError::Timeout)?
            .map_err(|_failure| RegistryError::Busy)?
    }
}
struct Owner {
    dir: DataDir,
    state: State,
}
impl Owner {
    async fn run(mut self, mut commands: mpsc::Receiver<Command>, stop: CancellationToken) {
        loop {
            let command = tokio::select! { biased; ()=stop.cancelled()=>return, command=commands.recv()=>command };
            match command {
                None => return,
                Some(Command::ListShared { reply }) => {
                    let _delivered = reply.send(self.state.shared());
                }
                Some(Command::List { source, reply }) => {
                    let _delivered = reply.send(self.state.list(source));
                }
                Some(Command::Change {
                    source,
                    change,
                    reply,
                }) => {
                    let result = self.commit(source, change).await;
                    let _delivered = reply.send(result);
                }
            }
        }
    }
    async fn commit(
        &mut self,
        source: HostSource,
        change: Change,
    ) -> Result<Option<HostRegistration>, RegistryError> {
        let (candidate, result) = self.state.prepare(source, change)?;
        let file = HostRegistryFile::new(candidate)?;
        let dir = self.dir.clone();
        let file = tokio::task::spawn_blocking(move || {
            dir.write_host_registry(&file)?;
            Ok::<_, RegistryFileError>(file)
        })
        .await
        .map_err(RegistryError::Worker)??;
        self.state = State::new(file);
        Ok(result)
    }
}

#[cfg(test)]
mod tests;
