//! Process-wide desktop ownership; screens use typed host adapters only.

mod persistence;
pub(crate) mod preferences;
mod startup;
mod startup_error;
#[cfg(test)]
mod testing;
mod windows;

use crate::shell::Shell;
use gpui::{AnyWindowHandle, App, Global, WeakEntity};
use std::sync::Arc;
use vessel_app_link::desktop::{
    environment::ShellEnvironment, preferences::Coordinator, store::Store,
    window_writer::WindowWriter,
};
use vessel_app_view::desktop::window::WindowState;

pub(crate) use persistence::{StateReadiness, StateReceipt, flush_current, flush_state};
pub(crate) use startup::run;
pub(crate) use windows::open_additional;

#[derive(Clone)]
struct WindowRecord {
    window: AnyWindowHandle,
    shell: WeakEntity<Shell>,
    close: WindowClose,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WindowClose {
    Idle,
    Checking,
}

pub(crate) type EnvironmentBroadcast = fn() -> Result<(), std::io::Error>;

type WindowCloseHandler = fn(&mut Shell, &mut gpui::Context<'_, Shell>) -> bool;

pub(crate) struct Desktop {
    records: Vec<WindowRecord>,
    saved: Option<WindowState>,
    latest: WindowState,
    geometry_publication: Arc<()>,
    writer: Option<WindowWriter>,
    environment: ShellEnvironment,
    environment_broadcast: Option<EnvironmentBroadcast>,
    quit: Option<fn(&mut App)>,
    window_close: Option<WindowCloseHandler>,
    session: Option<SessionHandler>,
    project: Option<ProjectHandler>,
    preferences: Coordinator,
}
impl Global for Desktop {}

impl Desktop {
    fn new(
        store: Store,
        environment: ShellEnvironment,
        environment_broadcast: Option<EnvironmentBroadcast>,
    ) -> Result<Self, vessel_app_link::desktop::DesktopError> {
        let preferences = Coordinator::start(vessel_app_link::Prefs::load())?;
        let (saved, writable) = match store.load_window() {
            Ok(saved) => (saved, true),
            Err(error) => {
                tracing::warn!(%error,"window state is unreadable; keeping it unchanged");
                (None, false)
            }
        };
        let writer = if writable {
            match WindowWriter::start(store) {
                Ok(writer) => Some(writer),
                Err(error) => {
                    tracing::warn!(%error,"window persistence could not start");
                    None
                }
            }
        } else {
            None
        };
        Ok(Self {
            records: Vec::new(),
            saved,
            latest: saved.unwrap_or_default(),
            geometry_publication: Arc::new(()),
            writer,
            environment,
            environment_broadcast,
            quit: None,
            window_close: None,
            session: None,
            project: None,
            preferences,
        })
    }

    fn remember(&mut self, state: WindowState) {
        self.saved = Some(state);
        self.latest = state;
        // Value equality cannot fence A -> B -> A while B is still in the
        // blocking writer. Every input publication invalidates old receipts.
        self.geometry_publication = Arc::new(());
        if let Some(writer) = &self.writer
            && let Err(error) = writer.changed(state)
        {
            tracing::warn!(%error,"window position could not be queued for saving");
        }
    }
}

type SessionHandler = fn(
    &mut Shell,
    vessel_proto::NodeId,
    vessel_proto::SessionId,
    &mut gpui::Window,
    &mut gpui::Context<'_, Shell>,
);
/// Directory delivery acknowledges only opening the exact managed-local draft.
pub(crate) enum ProjectDelivery {
    /// The exact managed-local editable draft opened; no trust or registration occurred.
    Opened,
    /// Readiness/refusal is temporary; retry without repeated UI side effects.
    Pending,
}

type ProjectHandler = fn(
    &mut Shell,
    &std::path::Path,
    &mut gpui::Window,
    &mut gpui::Context<'_, Shell>,
) -> ProjectDelivery;

pub(crate) fn set_session_handler(handler: SessionHandler, cx: &mut App) {
    cx.global_mut::<Desktop>().session = Some(handler);
}
pub(crate) fn set_project_handler(handler: ProjectHandler, cx: &mut App) {
    cx.global_mut::<Desktop>().project = Some(handler);
}

pub(crate) fn drain_pending(
    shell: &mut Shell,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<'_, Shell>,
) {
    use vessel_app_view::desktop::NavigationTarget;
    if !shell.loaded {
        return;
    }
    let Some(desktop) = cx.try_global::<Desktop>() else {
        return;
    };
    let session = desktop.session;
    let project = desktop.project;
    if let Some(open) = session
        && let Some(NavigationTarget::Session { node, session }) =
            shell.areas.desktop.pending.take_session()
    {
        open(shell, node, session, window, cx);
    }
    if let Some(open) = project
        && let Some(directory) = shell.areas.desktop.pending.take_project()
    {
        match open(shell, directory.path(), window, cx) {
            ProjectDelivery::Opened => {}
            ProjectDelivery::Pending => shell.areas.desktop.pending.defer_project(directory),
        }
    }
}

/// The local-node controller installs its asynchronous quit boundary once ready.
pub(crate) fn set_quit_handler(handler: fn(&mut App), cx: &mut App) {
    if cx.has_global::<Desktop>() {
        cx.global_mut::<Desktop>().quit = Some(handler);
    }
}

/// The document owner admits additional/macOS window removal after Shell borrows end.
pub(crate) fn set_window_close_handler(handler: WindowCloseHandler, cx: &mut App) {
    if cx.has_global::<Desktop>() {
        cx.global_mut::<Desktop>().window_close = Some(handler);
    }
}

/// Every menu and last-window close takes the controller's boundary when installed.
pub(crate) fn request_quit(cx: &mut App) {
    match cx.try_global::<Desktop>().and_then(|desktop| desktop.quit) {
        Some(quit) => quit(cx),
        None => cx.quit(),
    }
}

/// Only explicitly owned child commands inherit the resolved login environment.
pub(crate) fn environment(cx: &App) -> ShellEnvironment {
    cx.try_global::<Desktop>()
        .map_or_else(ShellEnvironment::inherited, |desktop| {
            desktop.environment.clone()
        })
}

/// Capture a thread-safe platform callback before explicit background PATH work.
pub(crate) fn environment_broadcast(cx: &App) -> Option<EnvironmentBroadcast> {
    cx.try_global::<Desktop>()
        .and_then(|desktop| desktop.environment_broadcast)
}

fn install(
    store: Store,
    environment: ShellEnvironment,
    environment_broadcast: Option<EnvironmentBroadcast>,
    cx: &mut App,
) -> Result<(), vessel_app_link::desktop::DesktopError> {
    cx.set_global(Desktop::new(store, environment, environment_broadcast)?);
    set_session_handler(Shell::open_session_address, cx);
    crate::update::install(
        !cx.global::<Desktop>().preferences.current().manual_updates,
        cx,
    );
    cx.set_quit_mode(gpui::QuitMode::Explicit);
    vessel_app_kit::chrome::set_close_handler(windows::request_close, cx);
    crate::areas::desktop::listen(cx);
    cx.on_window_closed(|cx, closed| {
        cx.global_mut::<Desktop>()
            .records
            .retain(|record| record.window.window_id() != closed);
        if !cfg!(target_os = "macos") && cx.windows().is_empty() {
            request_quit(cx);
        }
    })
    .detach();
    cx.on_app_quit(|cx| {
        // GPUI cancels shutdown observers after 200ms. This is a backstop;
        // the quit owner must await flush_state before normal quit/restart.
        let flush = flush_state(cx);
        async move {
            match flush.await {
                Err(error) => {
                    tracing::warn!(%error,"desktop state could not finish saving before quit")
                }
                Ok(receipt) => {
                    if let Some(error) = receipt.failure() {
                        tracing::warn!(%error,"preferences could not be saved before quit");
                    }
                }
            }
        }
    })
    .detach();
    Ok(())
}

/// Update restart and normal quit await the same canonical preferences queue.
pub(crate) fn flush_preferences(
    cx: &App,
) -> impl Future<Output = Result<(), vessel_app_link::desktop::DesktopError>> + 'static + use<> {
    let flush = cx
        .try_global::<Desktop>()
        .map(|desktop| desktop.preferences.flush());
    async move {
        match flush {
            Some(flush) => flush.await,
            None => Ok(()),
        }
    }
}

pub(crate) fn with_focused_shell(
    cx: &mut App,
    run: impl FnOnce(&mut Shell, &mut gpui::Window, &mut gpui::Context<'_, Shell>),
) {
    let record = target_window(cx);
    if let Some(record) = record {
        let result = record.window.update(cx, |_root, window, cx| {
            if let Some(shell) = record.shell.upgrade() {
                shell.update(cx, |shell, cx| run(shell, window, cx));
            }
        });
        if let Err(error) = result {
            tracing::warn!(%error,"desktop menu target is no longer available");
        }
    }
}

/// Quit admission reads document owners through the existing window registry.
/// Closed windows and expired entities grant no retained document target.
pub(crate) fn shells(cx: &App) -> Vec<WeakEntity<Shell>> {
    let Some(desktop) = cx.try_global::<Desktop>() else {
        return Vec::new();
    };
    let windows = cx.windows();
    desktop
        .records
        .iter()
        .filter(|record| windows.contains(&record.window))
        .filter_map(|record| record.shell.upgrade().map(|shell| shell.downgrade()))
        .collect()
}

/// Routing prefers the active or visible live window before a hidden first one.
fn target_window(cx: &mut App) -> Option<WindowRecord> {
    let active = cx.active_window();
    let live = cx.windows();
    let records: Vec<_> = cx
        .try_global::<Desktop>()?
        .records
        .iter()
        .filter(|record| live.contains(&record.window) && record.shell.upgrade().is_some())
        .cloned()
        .collect();
    records
        .iter()
        .find(|record| Some(record.window) == active)
        .or_else(|| {
            records.iter().find(|record| {
                record
                    .window
                    .update(cx, |_root, window, _cx| window.is_visible())
                    .unwrap_or(false)
            })
        })
        .or_else(|| records.first())
        .cloned()
}
