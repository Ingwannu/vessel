//! The updater as the app runs it: a task that checks the feed at start
//! and on a timer, downloads and installs a newer build, and publishes
//! what it is doing for the UI to show.
//!
//! Zed's model, on Vessel's types: check soon after start and then every
//! hour; download and install in the background; tell the owner "restart
//! to update" and let them choose when; stop checking once a build is
//! installed (a second update over the first gains nothing until the
//! restart). The UI reads [`UpdateState`] from a `watch`, so it always
//! sees the latest state and never a queue of old ones; it asks for a
//! check through a channel of one slot (a second ask while one waits adds
//! nothing).

use std::{ffi::OsString, sync::Arc, time::Duration};

use tokio::sync::{mpsc, watch};

use crate::{
    channel::Channel,
    feed::{FeedClient, FeedUrl, FetchError},
    install::{InstallError, Installation},
    key::PublicKey,
    label::Label,
    manifest::{ByteCount, Manifest, Notes, OpenError},
    platform::{Packaging, Target},
    restart::{RestartError, relaunch},
    stage::{self, StageError},
};

/// How often the feed is read: Zed's hour.
pub const EVERY: Duration = Duration::from_hours(1);

/// How long after start the first check waits, so it does not compete
/// with the app's first frames and its first connection to a node.
pub const FIRST_AFTER: Duration = Duration::from_secs(5);

/// What the updater needs to know, all of it fixed when the app is built
/// ([`crate::settings_from_build`]) except the timing.
#[derive(Debug, Clone)]
pub struct Settings {
    /// The feed's base.
    pub feed: FeedUrl,
    /// The release key's public half.
    pub key: PublicKey,
    /// The channel followed.
    pub channel: Channel,
    /// The running build's label.
    pub running: Label,
    /// The running build's target.
    pub target: Target,
    /// Time between checks.
    pub every: Duration,
    /// Time from start to the first check.
    pub first_after: Duration,
}

/// What the updater is doing, for the UI.
#[derive(Debug, Clone)]
pub enum UpdateState {
    /// Not checked yet.
    Idle,
    /// Reading the feed.
    Checking,
    /// The feed's newest build is not newer than this one.
    UpToDate {
        /// The feed's newest label.
        latest: Label,
    },
    /// Downloading a newer build.
    Downloading {
        /// Its label.
        label: Label,
        /// Bytes so far.
        received: ByteCount,
        /// Its size.
        total: ByteCount,
    },
    /// A newer build is installed; it runs from the next start.
    Ready {
        /// Its label.
        label: Label,
        /// Its release notes.
        notes: Option<Notes>,
    },
    /// The last attempt failed; the next check tries again.
    Failed {
        /// Why.
        error: Arc<UpdateError>,
    },
}

/// Why an attempt to update failed.
#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    /// The feed could not be read.
    #[error(transparent)]
    Fetch(#[from] FetchError),
    /// The feed's manifest was refused.
    #[error(transparent)]
    Open(#[from] OpenError),
    /// The feed has a newer build, but none for this system and packaging.
    #[error("{latest} has no build for {target} ({packaging})")]
    NoBuild {
        /// The newer label.
        latest: Label,
        /// This system.
        target: Target,
        /// This copy's packaging.
        packaging: Packaging,
    },
    /// The download failed or was not the signed bytes.
    #[error(transparent)]
    Stage(#[from] StageError),
    /// The swap failed; the app is as it was.
    #[error(transparent)]
    Install(#[from] InstallError),
    /// The swap's thread did not finish (the app is quitting).
    #[error("the update was interrupted")]
    Interrupted,
}

/// Why [`Updater::restart`] did not start the new build.
#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
    /// No newer build is installed yet.
    #[error("no update is ready")]
    NotReady,
    /// The new build did not start.
    #[error(transparent)]
    Restart(#[from] RestartError),
}

/// The handle the app keeps; dropping it stops the task.
#[derive(Debug)]
pub struct Updater {
    state: watch::Receiver<UpdateState>,
    commands: mpsc::Sender<()>,
    installation: Installation,
    running: Label,
}

impl Updater {
    /// Start the task on `runtime`.
    ///
    /// # Errors
    /// [`FetchError`] when the HTTPS client cannot be made; the app then
    /// runs without updates.
    pub fn start(
        settings: Settings,
        installation: Installation,
        runtime: &tokio::runtime::Handle,
    ) -> Result<Self, FetchError> {
        let client = FeedClient::new()?;
        let (state_tx, state) = watch::channel(UpdateState::Idle);
        let (commands, commands_rx) = mpsc::channel(1);
        let running = settings.running.clone();
        let task = Task {
            settings,
            installation: installation.clone(),
            client,
            state: state_tx,
        };
        drop(runtime.spawn(task.run(commands_rx)));
        Ok(Self {
            state,
            commands,
            installation,
            running,
        })
    }

    /// What the updater is doing; `changed()` on it wakes the UI.
    #[must_use]
    pub fn state(&self) -> watch::Receiver<UpdateState> {
        self.state.clone()
    }

    /// The running build's label.
    #[must_use]
    pub fn running(&self) -> &Label {
        &self.running
    }

    /// Ask for a check now. `false` when one is already waiting, or the
    /// task has stopped (an update is installed).
    #[must_use]
    pub fn check_now(&self) -> bool {
        self.commands.try_send(()).is_ok()
    }

    /// Start the installed update with `args` (the running process's
    /// arguments after the first). On `Ok` the app must quit.
    ///
    /// # Errors
    /// [`ApplyError::NotReady`] before an update is installed;
    /// [`ApplyError::Restart`] when it does not start.
    pub fn restart(&self, args: impl IntoIterator<Item = OsString>) -> Result<(), ApplyError> {
        let ready = matches!(*self.state.borrow(), UpdateState::Ready { .. });
        if !ready {
            return Err(ApplyError::NotReady);
        }
        relaunch(&self.installation, args, &self.running)?;
        Ok(())
    }
}

/// The task's own state.
struct Task {
    settings: Settings,
    installation: Installation,
    client: FeedClient,
    state: watch::Sender<UpdateState>,
}

impl Task {
    async fn run(self, mut commands: mpsc::Receiver<()>) {
        let installation = self.installation.clone();
        if tokio::task::spawn_blocking(move || installation.clean_leftovers())
            .await
            .is_err()
        {
            tracing::debug!("cleaning update leftovers was interrupted");
        }
        let first = tokio::time::Instant::now()
            .checked_add(self.settings.first_after)
            .unwrap_or_else(tokio::time::Instant::now);
        let mut ticks = tokio::time::interval_at(first, self.settings.every);
        // A machine that slept through checks checks once on waking, not
        // once per missed hour.
        ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = ticks.tick() => {}
                command = commands.recv() => {
                    if command.is_none() {
                        return;
                    }
                }
            }
            let _previous = self.state.send_replace(UpdateState::Checking);
            match self.attempt().await {
                Ok(done @ UpdateState::Ready { .. }) => {
                    tracing::info!("an update is installed and runs from the next start");
                    let _previous = self.state.send_replace(done);
                    return;
                }
                Ok(state) => {
                    let _previous = self.state.send_replace(state);
                }
                Err(error) => {
                    tracing::warn!(error = %error, "checking for an update failed");
                    let _previous = self.state.send_replace(UpdateState::Failed {
                        error: Arc::new(error),
                    });
                }
            }
        }
    }

    /// One check, and if the feed has a newer build, its download and
    /// install.
    async fn attempt(&self) -> Result<UpdateState, UpdateError> {
        let settings = &self.settings;
        let envelope = self
            .client
            .envelope(&settings.feed, &settings.channel)
            .await?;
        let manifest = Manifest::open(&envelope, &settings.key, &settings.channel)?;
        if manifest.label <= settings.running {
            return Ok(UpdateState::UpToDate {
                latest: manifest.label,
            });
        }
        let packaging = self.installation.packaging();
        let Some(artifact) = manifest.artifact(settings.target, packaging) else {
            return Err(UpdateError::NoBuild {
                latest: manifest.label,
                target: settings.target,
                packaging,
            });
        };
        let label = manifest.label.clone();
        let total = artifact.size;
        let _previous = self.state.send_replace(UpdateState::Downloading {
            label: label.clone(),
            received: ByteCount::default(),
            total,
        });
        let staged = stage::download(
            &self.client,
            &settings.feed,
            artifact,
            &label,
            self.installation.staging_dir(),
            |received| {
                let _previous = self.state.send_replace(UpdateState::Downloading {
                    label: label.clone(),
                    received,
                    total,
                });
            },
        )
        .await?;
        let installation = self.installation.clone();
        let running = settings.running.clone();
        tokio::task::spawn_blocking(move || {
            let installed = installation.replace_with(&staged, &running);
            if installed.is_err()
                && let Err(error) = std::fs::remove_file(staged.path())
            {
                tracing::debug!(error = %error, "a staged update was left for the next start");
            }
            installed
        })
        .await
        .map_err(|_cancelled| UpdateError::Interrupted)??;
        Ok(UpdateState::Ready {
            label: manifest.label,
            notes: manifest.notes,
        })
    }
}
