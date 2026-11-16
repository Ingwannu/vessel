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
//!   its own (five seconds after start, then hourly). It knows a build's notes only in the process that
//!   downloaded it, and that process ends at the restart. Nothing may run
//!   in a build that cannot update (a development build).
//! - Alternatives: polling the state from the UI (the
//!   updater offers a watch, which wakes the UI only on a change); the
//!   notes read again from the feed after the restart (a request at every
//!   start for one line of text).
//! - Choice: the updater lives on the shell ([`Updates`]), on a one-thread
//!   runtime of its own, and a GPUI task follows its watch into
//!   `UpdateUi::state`. "Update automatically" is the updater's own
//!   switch (`Updater::set_automatic`, track R's): off, its timer does not
//!   check, and "Check for updates" still does. An installed build's notes
//!   go into the app's preferences when it is ready, and the next start of
//!   that build shows them once (`Shell::announce_update`).
//! - Cost: one thread while the app runs, in a build that updates.
//!
//! `VESSEL_APP_NO_UPDATE` (any value) turns updates off for one run.

use vessel_app_kit::ui::{AsyncApp, Context, WeakEntity};
use vessel_app_link::{Command, UpdateNotes};
pub(crate) use vessel_app_updater::UpdateState;
use vessel_app_updater::{ApplyError, Installation, Settings, Updater, settings_from_build};

use crate::shell::Shell;

/// The app owns the updater even after its first window is closed.
struct SharedUpdates(Option<std::rc::Rc<Updates>>);
impl gpui::Global for SharedUpdates {}

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
        };
        updates.run(automatic).then_some(updates)
    }

    /// Start the updater, its timer on when `automatic` (the person's
    /// preference: `Updater::set_automatic`); `false` when it could not
    /// start.
    fn run(&mut self, automatic: bool) -> bool {
        drop(self.updater.take());
        let Some(runtime) = &self.runtime else {
            return false;
        };
        match Updater::start(
            self.settings.clone(),
            self.installation.clone(),
            runtime.handle(),
        ) {
            Ok(updater) => {
                updater.set_automatic(automatic);
                self.updater = Some(updater);
                self.generation = self.generation.wrapping_add(1);
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
    if !cx.has_global::<SharedUpdates>() {
        cx.set_global(SharedUpdates(
            Updates::start(!shell.prefs.manual_updates).map(std::rc::Rc::new),
        ));
    }
    let Some(updates) = cx
        .global::<SharedUpdates>()
        .0
        .as_ref()
        .map(std::rc::Rc::clone)
    else {
        return;
    };
    shell.updates.runtime = Some(updates);
    follow_current(shell, cx);
}

/// Follow the state of the shell's current updater.
fn follow_current(shell: &Shell, cx: &mut Context<'_, Shell>) {
    let Some((state, generation)) = shell.updates.runtime.as_ref().and_then(|running| {
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

impl Shell {
    /// Canonical preference broadcasts change the shared timer without another save.
    pub(crate) fn sync_update_automatically(&self) {
        if let Some(updater) = self
            .updates
            .runtime
            .as_ref()
            .and_then(|updates| updates.updater.as_ref())
        {
            updater.set_automatic(!self.prefs.manual_updates);
        }
    }
    /// The followed updater's state changed.
    fn update_changed(&mut self, generation: u64, state: UpdateState, cx: &mut Context<'_, Self>) {
        let Some(updates) = &self.updates.runtime else {
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
            self.command(Command::SavePrefs(Box::new(self.prefs.clone())));
        }
        self.updates.state = state;
        self.updates.refresh(self.words);
        cx.notify();
    }

    /// Check for an update now ("Check for updates", "Try again").
    pub(crate) fn check_for_updates(&mut self, cx: &mut Context<'_, Self>) {
        let Some(updater) = self
            .updates
            .runtime
            .as_ref()
            .and_then(|updates| updates.updater.as_ref())
        else {
            return;
        };
        if updater.check_now() {
            self.updates.asked = true;
            cx.notify();
        }
    }

    /// "Update automatically" switched: the updater's timer on or off
    /// (a check asked for runs either way).
    pub(crate) fn set_update_automatically(&mut self, on: bool, cx: &mut Context<'_, Self>) {
        self.areas.about.automatic_turns = self.areas.about.automatic_turns.saturating_add(1);
        self.prefs.manual_updates = !on;
        self.command(Command::SavePrefs(Box::new(self.prefs.clone())));
        if let Some(updater) = self
            .updates
            .runtime
            .as_ref()
            .and_then(|updates| updates.updater.as_ref())
        {
            updater.set_automatic(on);
        }
        cx.notify();
    }

    /// "Restart to update": start the installed build with this run's
    /// arguments, and quit.
    pub(crate) fn restart_to_update(&mut self, cx: &mut Context<'_, Self>) {
        self.restart_update_after(None, cx);
    }

    pub(crate) fn restart_to_update_confirmed(
        &mut self,
        confirmation: vessel_app_link::local_node::Confirmation,
        cx: &mut Context<'_, Self>,
    ) {
        self.restart_update_after(Some(confirmation), cx);
    }

    fn restart_update_after(
        &mut self,
        confirmation: Option<vessel_app_link::local_node::Confirmation>,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(runtime) = self.updates.runtime.as_ref().map(std::rc::Rc::clone) else {
            return;
        };
        if runtime.updater.is_none() || !crate::local_node::begin_finalization(cx) {
            return;
        }
        let finalize = crate::local_node::finalize(
            vessel_app_view::local_node::AfterQuit::Stop,
            confirmation,
            cx,
        );
        cx.spawn(async move |this, cx| {
            if !cx.update(crate::quit::activate) {
                return;
            }
            let before = crate::local_node::flush_then(cx, crate::local_node::admit_documents).await;
            match before {
                Ok(true) => {}
                Ok(false) => {
                    cx.update(crate::local_node::end_finalization);
                    return;
                }
                Err(error) => {
                    tracing::warn!(%error, "update restart could not settle desktop state");
                    cx.update(crate::local_node::end_finalization);
                    let _closed = this.update(cx, |shell, cx| {
                        shell.notice = Some(shell.words.quit().state_unsaved().to_owned());
                        cx.notify();
                    });
                    return;
                }
            }
            let result = finalize.await;
            if let Err(error) = result {
                tracing::warn!(%error, "update restart preserved the owned local node");
                cx.update(crate::local_node::end_finalization);
                let _closed = this.update(cx, |shell, cx| {
                    shell.local_node_update_refused(error.failure_snapshot().cloned());
                    shell.notice = Some(shell.updates.restart_failed(shell.words).to_string());
                    cx.notify();
                });
                return;
            }
            let saved = crate::local_node::flush_then(cx, |cx| {
                // No Shell entity is mutably borrowed during the all-window
                // guard, and there is no await before spawning the update.
                if !crate::local_node::admit_documents(cx) {
                    crate::local_node::end_finalization(cx);
                    return;
                }
                let Some(updater) = runtime.updater.as_ref() else {
                    crate::local_node::end_finalization(cx);
                    return;
                };
                match updater.restart(std::env::args_os().skip(1)) {
                    Ok(()) => cx.quit(),
                    Err(ApplyError::NotReady) => crate::local_node::end_finalization(cx),
                    Err(ApplyError::Restart(error)) => {
                        crate::local_node::end_finalization(cx);
                        tracing::warn!(error = %vessel_app_updater::chain(&error), "the updated app did not start");
                        let _closed = this.update(cx, |shell, cx| {
                            shell.notice = Some(shell.updates.restart_failed(shell.words).to_string());
                            cx.notify();
                        });
                    }
                }
            }).await;
            if let Err(error) = saved {
                tracing::warn!(%error, "update restart could not settle desktop state");
                cx.update(crate::local_node::end_finalization);
                let _closed = this.update(cx, |shell, cx| {
                    shell.notice = Some(shell.words.quit().state_unsaved().to_owned());
                    cx.notify();
                });
            }
        }).detach();
    }

    /// Whether this build updates itself.
    pub(crate) const fn updates_on(&self) -> bool {
        self.updates.runtime.is_some()
    }

    /// The running build's release notes, when the feed gave them (kept
    /// from the build that downloaded it).
    pub(crate) fn running_notes(&self) -> Option<&str> {
        self.prefs
            .update_notes
            .as_ref()
            .filter(|kept| kept.label == crate::LABEL)
            .map(|kept| kept.notes.as_str())
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
        // The running build's notes stay for About's "새로운 소식"; another
        // build's are dropped.
        if self
            .prefs
            .update_notes
            .as_ref()
            .is_some_and(|kept| kept.label != label)
        {
            self.prefs.update_notes = None;
        }
        let notes = self
            .prefs
            .update_notes
            .as_ref()
            .map(|kept| kept.notes.clone());
        if restarted || changed {
            self.updates.announcement = Some(Updated {
                label: label.to_owned(),
                notes,
            });
        }
        self.updates.refresh(self.words);
        self.prefs.last_label = Some(label.to_owned());
        self.command(Command::SavePrefs(Box::new(self.prefs.clone())));
    }
}

/// The note shown once after an update.
#[derive(Debug, Clone)]
pub(crate) struct Updated {
    pub(crate) label: String,
    pub(crate) notes: Option<String>,
}
