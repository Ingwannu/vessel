//! Host skill administration: one bounded, cancellation-independent blocking writer.
//! Explicit `HarnessEnv` roots keep library nodes and test fixtures out of real homes.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tokio::sync::Semaphore;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use vessel_fleet::orchestration::{
    SelectionError, SelectionOwnerError, SkillBundles, SkillError, SkillLocations, SkillManager,
};
use vessel_proto::skills::{
    ImportSkillsSelection, SaveSkillsSelection, SkillImportOutcome, SkillSelection, SkillsImported,
    SkillsSaved, SkillsSnapshot,
};
use vessel_store::{DataDir, StoreError};

use crate::config::Config;

mod task_error;
pub(crate) use task_error::SkillTaskFailure;

const MAX_COMMANDS: usize = 8;

#[derive(Debug, thiserror::Error)]
pub(crate) enum SkillsError {
    #[error("too many skill commands are already admitted")]
    Busy,
    #[error("the node is stopping")]
    Stopping,
    #[error("skill worker failed")]
    Lost(#[source] SkillTaskFailure),
    #[error("skill worker state is unavailable")]
    Poisoned,
    #[error(transparent)]
    Skill(#[from] SkillError),
    #[error("skill selection could not be read or kept")]
    Store(#[source] SelectionOwnerError<StoreError>),
    #[error(transparent)]
    Selection(#[from] SelectionError<StoreError>),
}

impl From<StoreError> for SkillsError {
    fn from(error: StoreError) -> Self {
        Self::Store(error.into())
    }
}

impl From<tokio::task::JoinError> for SkillsError {
    fn from(error: tokio::task::JoinError) -> Self {
        Self::Lost(error.into())
    }
}

#[derive(Debug)]
struct State {
    shared: PathBuf,
    claude: PathBuf,
    codex: PathBuf,
    data: DataDir,
    manager: Option<SkillManager>,
}
impl State {
    fn opened(
        &mut self,
    ) -> Result<(&mut SkillManager, DataDir, Option<SkillSelection>), SkillsError> {
        if self.manager.is_none() {
            let roots = SkillLocations::new(
                self.shared.clone(),
                self.claude.clone(),
                self.codex.clone(),
                std::fs::canonicalize(self.data.root())
                    .map_err(SkillError::from)?
                    .join("skill-transactions"),
            )?;
            self.manager = Some(SkillManager::new(SkillBundles::shipped()?, roots));
        }
        let durable = self.data.read_skills_selection()?;
        let manager = self.manager.as_mut().ok_or(SkillsError::Poisoned)?;
        Ok((manager, self.data.clone(), durable))
    }
}

#[derive(Debug)]
pub(crate) struct Skills {
    state: Arc<Mutex<State>>,
    permits: Arc<Semaphore>,
    tasks: TaskTracker,
    stop: CancellationToken,
}
impl Skills {
    pub(crate) fn start(config: &Config, tasks: TaskTracker, stop: CancellationToken) -> Arc<Self> {
        let env = &config.harness_env;
        let home = env.user_home();
        let harness = |name, fallback| {
            env.var(name)
                .map_or_else(|| home.join(fallback), PathBuf::from)
        };
        let skills = Arc::new(Self {
            state: Arc::new(Mutex::new(State {
                shared: home.join(".agents/skills"),
                claude: harness("CLAUDE_CONFIG_DIR", ".claude").join("skills"),
                codex: harness("CODEX_HOME", ".codex").join("skills"),
                data: config.data_dir.clone(),
                manager: None,
            })),
            permits: Arc::new(Semaphore::new(MAX_COMMANDS)),
            tasks,
            stop,
        });
        let maintenance = Arc::clone(&skills);
        drop(skills.tasks.spawn(async move {
            if let Err(error) = maintenance
                .run(|manager, _data, durable| {
                    manager.maintain(durable.as_ref()).map_err(Into::into)
                })
                .await
            {
                tracing::warn!(%error, "installed orchestration skills could not be maintained");
            }
        }));
        skills
    }

    async fn run<T: Send + 'static>(
        self: &Arc<Self>,
        action: impl FnOnce(
            &mut SkillManager,
            DataDir,
            Option<SkillSelection>,
        ) -> Result<T, SkillsError>
        + Send
        + 'static,
    ) -> Result<T, SkillsError> {
        let permit = Arc::clone(&self.permits)
            .try_acquire_owned()
            .map_err(|_full| SkillsError::Busy)?;
        let admitted = self.tasks.token();
        if self.stop.is_cancelled() {
            return Err(SkillsError::Stopping);
        }
        let state = Arc::clone(&self.state);
        self.tasks
            .spawn(async move {
                let (_permit, _admitted) = (permit, admitted);
                tokio::task::spawn_blocking(move || {
                    let mut state = state.lock().map_err(|_poison| SkillsError::Poisoned)?;
                    let (manager, data, durable) = state.opened()?;
                    action(manager, data, durable)
                })
                .await
                .map_err(SkillsError::from)?
            })
            .await
            .map_err(SkillsError::from)?
    }
    pub(crate) async fn status(self: &Arc<Self>) -> Result<SkillsSnapshot, SkillsError> {
        self.run(|manager, _data, durable| manager.status(durable.as_ref()).map_err(Into::into))
            .await
    }
    pub(crate) async fn reconcile(self: &Arc<Self>) -> Result<SkillsSnapshot, SkillsError> {
        self.run(|manager, _data, durable| manager.reconcile(durable.as_ref()).map_err(Into::into))
            .await
    }
    pub(crate) async fn uninstall(self: &Arc<Self>) -> Result<SkillsSnapshot, SkillsError> {
        self.run(|manager, _data, durable| manager.uninstall(durable.as_ref()).map_err(Into::into))
            .await
    }
    pub(crate) async fn save(
        self: &Arc<Self>,
        request: SaveSkillsSelection,
    ) -> Result<SkillsSaved, SkillsError> {
        self.run(move |manager, data, durable| {
            Ok(manager.save(
                durable,
                &request.selection,
                &request.confirmed_removals,
                |next| data.write_skills_selection(next, chrono::Utc::now()),
            )?)
        })
        .await
    }
    pub(crate) async fn import(
        self: &Arc<Self>,
        request: ImportSkillsSelection,
    ) -> Result<SkillsImported, SkillsError> {
        self.run(move |manager, data, durable| {
            let _snapshot = manager.status(durable.as_ref())?;
            if let Some(selection) = durable {
                return Ok(SkillsImported {
                    selection,
                    outcome: SkillImportOutcome::AlreadySet,
                });
            }
            manager.validate_selection(&request.selection)?;
            data.write_skills_selection(&request.selection, chrono::Utc::now())?;
            Ok(SkillsImported {
                selection: request.selection,
                outcome: SkillImportOutcome::Imported,
            })
        })
        .await
    }
}

#[cfg(test)]
mod tests;
