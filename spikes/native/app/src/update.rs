//! The app's updater (spikes/release/updater), as Zed's behaves: it checks
//! shortly after start and then hourly, downloads in the background, and
//! leaves the moment of restart to the person; a check can be asked for at
//! any time; automatic updates can be turned off (then only that check
//! runs); after an update the app says once what it was updated to; a
//! failed check or download is quiet and tried again at the next check.
//!
//! [Decision Log]
//! - Purpose: the owner asked for automatic updates "as Zed has them".
//! - Constraints: the updater runs on tokio, GPUI on its own executor; the
//!   title bar that shows the state is the desktop track's to draw; the
//!   app has no preferences store of its own yet; a build without a feed
//!   compiled in (a development build) must not update or spend anything.
//! - Choice: one GPUI entity, [`Updates`], set as a global so any view can
//!   read and observe it: its [`UpdateStatus`], [`Updates::check_now`],
//!   [`Updates::restart`], [`Updates::set_automatic`] and, once,
//!   [`Updates::take_updated_to`]. A one-thread tokio runtime of its own
//!   runs the updater, created only when the build can update. The
//!   preference and the last build run are the updater's `Memory`, a small
//!   file beside the node book (`<config>/vessel/app-updates.json`).
//! - Why: the drawing and the state stay apart (the lead draws from this
//!   entity, nothing else); the link's runtime stays the link's.
//! - Cost: one more thread while updates are on. Until the title bar draws
//!   the state, the frame's foot carries two sentences: the update
//!   installed, and the build updated to.
//!
//! Where it differs from Zed: Zed's Linux and macOS builds replace the app
//! on disk the same way, but on Windows Zed runs its installer through a
//! helper after the app exits; here the running executable is renamed
//! aside and the new one put in place (spikes/release/updater, install.rs).
//!
//! `VESSEL_APP_NO_UPDATE` (any value) turns the updater off for one run.

use std::path::PathBuf;

use gpui::{App, AppContext as _, AsyncApp, Context, Entity, Global, WeakEntity};
use vessel_app_updater::{
    ApplyError, Installation, Label, Memory, UpdateState, Updater, build_label, settings_from_build,
};

use crate::shell::Shell;

/// What the title bar shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UpdateStatus {
    /// This build does not update itself (no feed compiled in, or turned
    /// off for this run).
    Off,
    /// Waiting for the next check.
    Idle,
    /// Reading the feed.
    Checking,
    /// The feed's newest build is this one or older.
    UpToDate {
        /// The feed's newest label.
        latest: Label,
    },
    /// Downloading a newer build.
    Downloading {
        /// Its label.
        label: Label,
        /// Bytes so far.
        received: u64,
        /// Its size.
        total: u64,
    },
    /// A newer build is in place: "restart to update".
    Ready {
        /// Its label.
        label: Label,
    },
    /// The last check or download failed; the next check tries again.
    Failed {
        /// Why, in a sentence.
        reason: String,
    },
}

/// The updater as the views see it.
#[expect(
    dead_code,
    reason = "the title bar (the desktop track's) reads these; remove when it does"
)]
pub(crate) struct Updates {
    status: UpdateStatus,
    automatic: bool,
    updated_to: Option<Label>,
    updater: Option<Updater>,
    memory: Option<(PathBuf, Memory)>,
    runtime: Runtime,
}

/// The entity, as a global.
#[expect(
    dead_code,
    reason = "read through Updates::global by the title bar (the desktop track's)"
)]
struct UpdatesGlobal(Entity<Updates>);

impl Global for UpdatesGlobal {}

#[expect(
    dead_code,
    reason = "the title bar (the desktop track's) calls these; remove when it does"
)]
impl Updates {
    /// The app's updater, once [`start`] has run.
    pub(crate) fn global(cx: &App) -> Option<Entity<Self>> {
        cx.try_global::<UpdatesGlobal>()
            .map(|global| global.0.clone())
    }

    /// What to show.
    pub(crate) fn status(&self) -> &UpdateStatus {
        &self.status
    }

    /// Whether the app checks by itself (the person's preference).
    pub(crate) fn automatic(&self) -> bool {
        self.automatic
    }

    /// Check now ("Check for updates"). Does nothing when this build does
    /// not update or a build is already installed.
    pub(crate) fn check_now(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(updater) = &self.updater
            && updater.check_now()
        {
            self.status = UpdateStatus::Checking;
            cx.notify();
        }
    }

    /// Turn automatic updates on or off, and remember it.
    pub(crate) fn set_automatic(&mut self, on: bool, cx: &mut Context<'_, Self>) {
        self.automatic = on;
        if let Some(updater) = &self.updater {
            updater.set_automatic(on);
        }
        if let Some((path, memory)) = &mut self.memory {
            memory.automatic = on;
            save(path.clone(), memory.clone(), cx);
        }
        cx.notify();
    }

    /// Start the installed build and quit ("Restart to update").
    ///
    /// # Errors
    /// [`ApplyError::NotReady`] before a build is installed, or when this
    /// build does not update; [`ApplyError::Restart`] when the new build
    /// does not start (the app then stays open).
    pub(crate) fn restart(&mut self, cx: &mut Context<'_, Self>) -> Result<(), ApplyError> {
        let updater = self.updater.as_ref().ok_or(ApplyError::NotReady)?;
        updater.restart(std::env::args_os().skip(1))?;
        cx.quit();
        Ok(())
    }

    /// The build this one was updated to, the first time it is asked after
    /// an update; `None` after that.
    pub(crate) fn take_updated_to(&mut self) -> Option<Label> {
        self.updated_to.take()
    }
}

/// Start the updater, and set [`Updates`] as a global.
pub(crate) fn start(shell: WeakEntity<Shell>, cx: &mut App) {
    let memory = memory_path().map(|path| {
        let mut memory = Memory::load(&path);
        let updated_to = build_label().and_then(|running| memory.note_run(&running));
        (path, memory, updated_to)
    });
    let (memory, updated_to) = match memory {
        Some((path, memory, updated_to)) => {
            save(path.clone(), memory.clone(), cx);
            (Some((path, memory)), updated_to)
        }
        None => (None, None),
    };
    let automatic = memory.as_ref().is_none_or(|(_, memory)| memory.automatic);
    let (updater, runtime) = updater(automatic);
    let status = if updater.is_some() {
        UpdateStatus::Idle
    } else {
        UpdateStatus::Off
    };
    let state = updater.as_ref().map(Updater::state);
    let announce = updated_to.clone();
    let entity = cx.new(|_| Updates {
        status,
        automatic,
        updated_to,
        updater,
        memory,
        runtime,
    });
    cx.set_global(UpdatesGlobal(entity.clone()));
    if let Some(label) = announce {
        say(&shell, cx, move |shell| {
            shell.t.updated_to(&label.to_string())
        });
    }
    if let Some(state) = state {
        let weak = entity.downgrade();
        cx.spawn(async move |cx| follow(state, weak, shell, cx).await)
            .detach();
    }
}

/// The updater and its runtime, when this build can update.
fn updater(automatic: bool) -> (Option<Updater>, Runtime) {
    if std::env::var_os("VESSEL_APP_NO_UPDATE").is_some() {
        tracing::info!("updates are off for this run");
        return (None, Runtime(None));
    }
    let mut settings = match settings_from_build() {
        Ok(Some(settings)) => settings,
        Ok(None) => return (None, Runtime(None)),
        Err(error) => {
            tracing::warn!(%error, "this build's update settings do not parse; no updates");
            return (None, Runtime(None));
        }
    };
    settings.automatic = automatic;
    let installation = match Installation::from_process(settings.target.default_packaging()) {
        Ok(installation) => installation,
        Err(error) => {
            tracing::warn!(%error, "the app could not find itself on disk; no updates");
            return (None, Runtime(None));
        }
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .thread_name("vessel-update")
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            tracing::warn!(%error, "the updater's runtime could not start; no updates");
            return (None, Runtime(None));
        }
    };
    match Updater::start(settings, installation, runtime.handle()) {
        Ok(updater) => (Some(updater), Runtime(Some(runtime))),
        Err(error) => {
            tracing::warn!(%error, "the updater could not start; no updates");
            (None, Runtime(Some(runtime)))
        }
    }
}

/// Mirror the updater's state into the entity; say when a build is
/// installed (until the title bar draws it).
async fn follow(
    mut state: tokio::sync::watch::Receiver<UpdateState>,
    updates: WeakEntity<Updates>,
    shell: WeakEntity<Shell>,
    cx: &mut AsyncApp,
) {
    loop {
        let now = state.borrow_and_update().clone();
        let status = match now {
            UpdateState::Idle => UpdateStatus::Idle,
            UpdateState::Checking => UpdateStatus::Checking,
            UpdateState::UpToDate { latest } => UpdateStatus::UpToDate { latest },
            UpdateState::Downloading {
                label,
                received,
                total,
            } => UpdateStatus::Downloading {
                label,
                received: received.get(),
                total: total.get(),
            },
            UpdateState::Ready { label, .. } => UpdateStatus::Ready { label },
            UpdateState::Failed { error } => UpdateStatus::Failed {
                reason: error.to_string(),
            },
        };
        if let UpdateStatus::Ready { label } = &status {
            let label = label.to_string();
            let said = shell.update(cx, |shell, cx| {
                shell.notice = Some(shell.t.update_installed(&label));
                cx.notify();
            });
            if said.is_err() {
                return;
            }
        }
        let mirrored = updates.update(cx, |updates, cx| {
            updates.status = status;
            cx.notify();
        });
        if mirrored.is_err() {
            return;
        }
        // The updater's task ends once a build is installed; the last state
        // stays, and nothing more will change.
        if state.changed().await.is_err() {
            return;
        }
    }
}

/// Put a sentence in the frame's foot.
fn say(shell: &WeakEntity<Shell>, cx: &mut App, text: impl FnOnce(&Shell) -> String) {
    if let Some(shell) = shell.upgrade() {
        shell.update(cx, |shell, cx| {
            shell.notice = Some(text(shell));
            cx.notify();
        });
    }
}

/// Where the updater's memory is: beside the node book, in the directory
/// the CLI and the link read (`VESSEL_CONFIG_DIR`, else the system's).
fn memory_path() -> Option<PathBuf> {
    std::env::var_os("VESSEL_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(dirs::config_dir)
        .map(|dir| dir.join("vessel").join("app-updates.json"))
}

/// Write the memory off the UI thread; a failure only loses the note.
fn save(path: PathBuf, memory: Memory, cx: &App) {
    cx.background_spawn(async move {
        if let Err(error) = memory.save(&path) {
            tracing::debug!(%error, "the update memory was not written");
        }
    })
    .detach();
}

/// The updater's runtime, stopped without waiting when the app quits (a
/// blocking shutdown would hold the UI thread on the way out).
struct Runtime(Option<tokio::runtime::Runtime>);

impl Drop for Runtime {
    fn drop(&mut self) {
        if let Some(runtime) = self.0.take() {
            runtime.shutdown_background();
        }
    }
}
