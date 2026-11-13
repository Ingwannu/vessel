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

/// After a failed check, the waits before the next ones: one, five and
/// fifteen minutes, then [`EVERY`] again. A lost connection or a file
/// held for a moment by a virus scanner passes in minutes; an hour (what
/// 0.0.0.0.3 waited) leaves the owner looking at no update for no reason.
/// A failure a new publish must fix (a manifest that does not verify, no
/// build for this machine) waits [`EVERY`] at once
/// ([`UpdateError::is_transient`]).
pub const RETRY_AFTER: [Duration; 3] = [
    Duration::from_mins(1),
    Duration::from_mins(5),
    Duration::from_mins(15),
];

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
    /// Whether the timer checks at all (Zed's `auto_update`); off leaves
    /// only [`Updater::check_now`]. Changed while running with
    /// [`Updater::set_automatic`].
    pub automatic: bool,
    /// The waits after consecutive failed checks, then [`Settings::every`]
    /// ([`RETRY_AFTER`]; tests use shorter ones).
    pub retry_after: Vec<Duration>,
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

impl UpdateError {
    /// Whether checking again soon can help: everything but a manifest
    /// that does not verify or parse and a missing build for this
    /// machine, which only a new publish changes.
    #[must_use]
    pub const fn is_transient(&self) -> bool {
        !matches!(self, Self::Open(_) | Self::NoBuild { .. })
    }
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
    automatic: watch::Sender<bool>,
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
        let (automatic, automatic_rx) = watch::channel(settings.automatic);
        let running = settings.running.clone();
        let task = Task {
            settings,
            installation: installation.clone(),
            client,
            state: state_tx,
        };
        drop(runtime.spawn(task.run(commands_rx, automatic_rx)));
        Ok(Self {
            state,
            commands,
            automatic,
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

    /// Turn the timer's checks on or off (the person's preference); a
    /// check asked for with [`Self::check_now`] runs either way.
    pub fn set_automatic(&self, on: bool) {
        let _previous = self.automatic.send_replace(on);
    }

    /// Whether the timer checks.
    #[must_use]
    pub fn automatic(&self) -> bool {
        *self.automatic.borrow()
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
    async fn run(self, mut commands: mpsc::Receiver<()>, automatic: watch::Receiver<bool>) {
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
        tracing::info!(
            running = %self.settings.running,
            channel = %self.settings.channel,
            target = %self.settings.target,
            packaging = %self.installation.packaging(),
            feed = %self.settings.feed,
            "the updater runs"
        );
        // Consecutive failed checks, for the wait before the next.
        let mut failures: usize = 0;
        loop {
            tokio::select! {
                _ = ticks.tick() => {
                    // Automatic updates off: the timer asks nothing; only
                    // a check the person asks for runs.
                    if !*automatic.borrow() {
                        continue;
                    }
                }
                command = commands.recv() => {
                    if command.is_none() {
                        return;
                    }
                }
            }
            let _previous = self.state.send_replace(UpdateState::Checking);
            tracing::info!("checking for an update");
            match self.attempt().await {
                Ok(done @ UpdateState::Ready { .. }) => {
                    tracing::info!("an update is installed and runs from the next start");
                    let _previous = self.state.send_replace(done);
                    return;
                }
                Ok(state) => {
                    if let UpdateState::UpToDate { latest } = &state {
                        tracing::info!(%latest, "up to date: the feed's newest is not newer");
                    }
                    failures = 0;
                    let _previous = self.state.send_replace(state);
                }
                Err(error) => {
                    let wait = if error.is_transient() {
                        self.settings
                            .retry_after
                            .get(failures)
                            .copied()
                            .unwrap_or(self.settings.every)
                    } else {
                        self.settings.every
                    };
                    failures = failures.saturating_add(1);
                    ticks.reset_after(wait);
                    tracing::warn!(
                        error = %crate::report::chain(&error),
                        failures,
                        next_check_in = ?wait,
                        "checking for an update failed"
                    );
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
        tracing::debug!(
            bytes = envelope.len(),
            latest = %manifest.label,
            "the manifest verifies"
        );
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
        tracing::info!(
            label = %manifest.label,
            artifact = %artifact.path,
            bytes = artifact.size.get(),
            "downloading the update"
        );
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
        tracing::info!(
            staged = %file_name(staged.path()),
            "the download is complete, verified and on disk"
        );
        let installation = self.installation.clone();
        let running = settings.running.clone();
        tokio::task::spawn_blocking(move || {
            let installed = installation.replace_with(&staged, &running);
            if installed.is_err()
                && let Err(error) = std::fs::remove_file(staged.path())
            {
                tracing::warn!(
                    error = %crate::report::chain(&error),
                    staged = %file_name(staged.path()),
                    "a staged update was left for the next start"
                );
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

/// A path's file name for the log: the directory around it is the
/// owner's (it carries their account's name on Windows) and adds nothing.
pub(crate) fn file_name(path: &std::path::Path) -> String {
    path.file_name().map_or_else(
        || "?".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    )
}
