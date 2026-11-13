//! The app's updates, as Zed shows them: the updater (track R's
//! `spikes/release/updater`) runs beside the app, its state is drawn in
//! the title bar (nothing while idle; "Checking for updates…";
//! "Downloading update… 42%"; "Restart to update" once a build is in
//! place; a quiet failure with "Try again"), the About screen checks now
//! and turns automatic updates off, and the first start of a new build
//! says "Updated to …" once, with the feed's notes.
//!
//! [Decision Log]
//! - Purpose: a build the owner installed finds the next one by itself
//!   and puts it in place, and the person sees it happen and chooses when
//!   to restart into it (the owner: "자동업데이트도 … ZED 에 있는것처럼").
//! - Constraints: the updater is track R's crate and is not changed here;
//!   it runs on tokio, GPUI on its own executor; it checks on a timer of
//!   its own (five seconds after start, then hourly) and has no switch for
//!   "only when asked". It knows a build's notes only in the process that
//!   downloaded it, and that process ends at the restart. Nothing may run
//!   in a build that cannot update (a development build).
//! - Alternatives: a pause flag in the updater (track R's to add; asked
//!   for through the coordinator); polling the state from the UI (the
//!   updater offers a watch, which wakes the UI only on a change); the
//!   notes read again from the feed after the restart (a request at every
//!   start for one line of text).
//! - Choice: the updater lives on the shell ([`Updates`]), on a one-thread
//!   runtime of its own, and a GPUI task follows its watch into
//!   `Shell::update_state`. "Update automatically" off starts it with a
//!   first check and a period of a year (the timing is the one part of its
//!   settings meant to be set by the app), so it checks only when asked;
//!   switching the preference starts a new updater with the other timing,
//!   once the current one is not checking or downloading (two would write
//!   the same staging file). An installed build's notes go into the app's
//!   preferences when it is ready, and the next start of that build shows
//!   them once (`Shell::announce_update`).
//! - Cost: one thread while the app runs, in a build that updates. A
//!   switch of the preference during a download takes effect after it.
//!
//! `VESSEL_APP_NO_UPDATE` (any value) turns updates off for one run.

use std::time::Duration;

use vessel_app_kit::ui::{AsyncApp, Context, WeakEntity};
use vessel_app_link::{Command, UpdateNotes};
pub(crate) use vessel_app_updater::UpdateState;
use vessel_app_updater::{ApplyError, Installation, Settings, Updater, settings_from_build};

use crate::shell::Shell;

/// The first check and the period when updates are only checked when
/// asked: far enough never to come while the app runs (a timer this long
/// is still within what tokio keeps).
const ONLY_WHEN_ASKED: Duration = Duration::from_hours(365 * 24);

/// The updater and what it runs on, owned by the shell.
pub(crate) struct Updates {
    /// Dropped before the runtime (its task runs there).
    updater: Option<Updater>,
    runtime: Option<tokio::runtime::Runtime>,
    settings: Settings,
    installation: Installation,
    /// Which updater the followed state is from: a replaced one's last
    /// states are ignored.
    generation: u64,
    /// Whether the running updater checks by itself.
    automatic: bool,
    /// The preference changed while the updater was busy: switch when it
    /// is not.
    wanted: Option<bool>,
}

impl Drop for Updates {
    fn drop(&mut self) {
        drop(self.updater.take());
        // Not a blocking drop: the updater's task may be in a request.
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_background();
        }
    }
}

impl Updates {
    /// The updater for this build, checking by itself when `automatic`;
    /// `None` when the build cannot update or updates are off for this run.
    fn start(automatic: bool) -> Option<Self> {
        if std::env::var_os("VESSEL_APP_NO_UPDATE").is_some() {
            tracing::info!("updates are off for this run");
            return None;
        }
        let settings = match settings_from_build() {
            Ok(Some(settings)) => settings,
            Ok(None) => return None,
            Err(error) => {
                tracing::warn!(%error, "this build's update settings do not parse; no updates");
                return None;
            }
        };
        let installation = match Installation::from_process(settings.target.default_packaging()) {
            Ok(installation) => installation,
            Err(error) => {
                tracing::warn!(%error, "the app could not find itself on disk; no updates");
                return None;
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
                return None;
            }
        };
        let mut updates = Self {
            updater: None,
            runtime: Some(runtime),
            settings,
            installation,
            generation: 0,
            automatic,
            wanted: None,
        };
        updates.run(automatic).then_some(updates)
    }

    /// Start an updater with the timing `automatic` asks for, in place of
    /// the one running; `false` when it could not start.
    fn run(&mut self, automatic: bool) -> bool {
        drop(self.updater.take());
        let Some(runtime) = &self.runtime else {
            return false;
        };
        let mut settings = self.settings.clone();
        if !automatic {
            settings.first_after = ONLY_WHEN_ASKED;
            settings.every = ONLY_WHEN_ASKED;
        }
        match Updater::start(settings, self.installation.clone(), runtime.handle()) {
            Ok(updater) => {
                self.updater = Some(updater);
                self.generation = self.generation.wrapping_add(1);
                self.automatic = automatic;
                true
            }
            Err(error) => {
                tracing::warn!(%error, "the updater could not start; no updates");
                false
            }
        }
    }
}

/// Start updates for this build, and follow them into the shell.
pub(crate) fn start(shell: &mut Shell, cx: &mut Context<'_, Shell>) {
    let Some(updates) = Updates::start(!shell.prefs.manual_updates) else {
        return;
    };
    shell.updates = Some(updates);
    follow_current(shell, cx);
}

/// Follow the state of the shell's current updater.
fn follow_current(shell: &Shell, cx: &mut Context<'_, Shell>) {
    let Some((state, generation)) = shell.updates.as_ref().and_then(|running| {
        running
            .updater
            .as_ref()
            .map(|updater| (updater.state(), running.generation))
    }) else {
        return;
    };
    cx.spawn(async move |this, cx| follow(state, generation, this, cx).await)
        .detach();
}

/// Follow one updater's state until it ends or the shell goes.
async fn follow(
    mut state: tokio::sync::watch::Receiver<UpdateState>,
    generation: u64,
    shell: WeakEntity<Shell>,
    cx: &mut AsyncApp,
) {
    loop {
        let now = state.borrow_and_update().clone();
        let applied = shell.update(cx, |shell, cx| shell.update_changed(generation, now, cx));
        if applied.is_err() {
            break;
        }
        // The updater's task ends once a build is installed (or the
        // updater is replaced); its last state stays as it was.
        if state.changed().await.is_err() {
            break;
        }
    }
}

/// Whether the updater is in the middle of something a replacement would
/// collide with.
const fn busy(state: &UpdateState) -> bool {
    matches!(
        state,
        UpdateState::Checking | UpdateState::Downloading { .. }
    )
}

impl Shell {
    /// The followed updater's state changed.
    fn update_changed(&mut self, generation: u64, state: UpdateState, cx: &mut Context<'_, Self>) {
        let Some(updates) = &self.updates else {
            return;
        };
        if updates.generation != generation {
            return;
        }
        if let UpdateState::Ready { label, notes } = &state {
            // Kept for the build they describe, which shows them once.
            self.prefs.update_notes = notes.as_ref().map(|notes| UpdateNotes {
                label: label.to_string(),
                notes: notes.as_str().to_owned(),
            });
            self.command(Command::SavePrefs(self.prefs.clone()));
        }
        let idle = !busy(&state) && !matches!(state, UpdateState::Ready { .. });
        self.update_state = state;
        if idle {
            self.switch_if_wanted(cx);
        }
        cx.notify();
    }

    /// Check for an update now ("Check for updates", "Try again").
    pub(crate) fn check_for_updates(&mut self, cx: &mut Context<'_, Self>) {
        let Some(updater) = self
            .updates
            .as_ref()
            .and_then(|updates| updates.updater.as_ref())
        else {
            return;
        };
        if updater.check_now() {
            self.update_asked = true;
            cx.notify();
        }
    }

    /// "Update automatically" switched.
    pub(crate) fn set_update_automatically(&mut self, on: bool, cx: &mut Context<'_, Self>) {
        self.prefs.manual_updates = !on;
        self.command(Command::SavePrefs(self.prefs.clone()));
        if let Some(updates) = &mut self.updates {
            updates.wanted = Some(on);
        }
        if !busy(&self.update_state) && !matches!(self.update_state, UpdateState::Ready { .. }) {
            self.switch_if_wanted(cx);
        }
        cx.notify();
    }

    /// Start the updater with the timing the preference asks for, when it
    /// differs from the running one's.
    fn switch_if_wanted(&mut self, cx: &mut Context<'_, Self>) {
        let Some(updates) = &mut self.updates else {
            return;
        };
        let Some(automatic) = updates.wanted.take() else {
            return;
        };
        if automatic == updates.automatic {
            return;
        }
        if updates.run(automatic) {
            self.update_state = UpdateState::Idle;
            follow_current(self, cx);
        } else {
            self.updates = None;
        }
    }

    /// "Restart to update": start the installed build with this run's
    /// arguments, and quit.
    pub(crate) fn restart_to_update(&mut self, cx: &mut Context<'_, Self>) {
        let Some(updater) = self
            .updates
            .as_ref()
            .and_then(|updates| updates.updater.as_ref())
        else {
            return;
        };
        // Written here, not through the link: the app quits next, before
        // the link's write would run.
        if let Err(error) = self.prefs.save() {
            tracing::warn!(%error, "the app's preferences could not be saved before the restart");
        }
        match updater.restart(std::env::args_os().skip(1)) {
            Ok(()) => cx.quit(),
            Err(ApplyError::NotReady) => {}
            Err(error) => {
                self.notice = Some(error.to_string());
                cx.notify();
            }
        }
    }

    /// Whether this build updates itself.
    pub(crate) const fn updates_on(&self) -> bool {
        self.updates.is_some()
    }

    /// Whether the running updater checks by itself.
    pub(crate) fn updates_automatic(&self) -> bool {
        !self.prefs.manual_updates
    }

    /// At start: when this build is not the one that ran last (or the
    /// updater restarted into it), say once that it was updated to, with
    /// its notes when they were kept; remember it as the one that ran.
    pub(crate) fn announce_update(&mut self) {
        let label = crate::LABEL;
        if label == "dev" {
            return;
        }
        let restarted = std::env::var_os(vessel_app_updater::RESTARTED_FROM).is_some();
        let changed = self
            .prefs
            .last_label
            .as_deref()
            .is_some_and(|last| last != label);
        let notes = self
            .prefs
            .update_notes
            .take()
            .filter(|kept| kept.label == label)
            .map(|kept| kept.notes);
        if restarted || changed {
            self.updated = Some(Updated {
                label: label.to_owned(),
                notes,
            });
        }
        self.prefs.last_label = Some(label.to_owned());
        self.command(Command::SavePrefs(self.prefs.clone()));
    }
}

/// The note shown once after an update.
#[derive(Debug, Clone)]
pub(crate) struct Updated {
    pub(crate) label: String,
    pub(crate) notes: Option<String>,
}

/// What the title bar shows for `state`, as a percentage when
/// downloading.
pub(crate) fn percent(received: u64, total: u64) -> u8 {
    if total == 0 {
        return 0;
    }
    let percent = received.saturating_mul(100).div_euclid(total);
    u8::try_from(percent.min(100)).unwrap_or(100)
}
