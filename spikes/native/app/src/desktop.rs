//! Process-wide desktop ownership; screens use typed host adapters only.

mod persistence;
pub(crate) mod preferences;
mod startup;
mod startup_error;
#[cfg(test)]
mod testing;
mod windows;

use crate::host_windows::{WindowClose, WindowRecord};
use crate::shell::Shell;
use gpui::{App, Global};
use std::sync::Arc;
use vessel_app_link::desktop::{
    environment::ShellEnvironment, preferences::Coordinator, store::Store,
    window_writer::WindowWriter,
};
use vessel_app_view::desktop::window::WindowState;

pub(crate) use persistence::{StateReadiness, StateReceipt, flush_current, flush_state};
pub(crate) use startup::run;
pub(crate) fn open_additional(cx: &mut App) {
    let _delivery = crate::host_windows::open_additional(cx);
}

type WindowCloseHandler = fn(&mut Shell, &mut gpui::Context<'_, Shell>) -> bool;

pub(crate) struct Desktop {
    saved: Option<WindowState>,
    latest: WindowState,
    geometry_publication: Arc<()>,
    writer: Option<WindowWriter>,
    environment: ShellEnvironment,
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
            saved,
            latest: saved.unwrap_or_default(),
            geometry_publication: Arc::new(()),
            writer,
            environment,
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
        // Every publication fences A -> B -> A while B may still be in flight.
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
type ProjectHandler =
    fn(&mut Shell, &std::path::Path, &mut gpui::Window, &mut gpui::Context<'_, Shell>);

pub(crate) fn drain_pending(
    shell: &mut Shell,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<'_, Shell>,
) {
    use vessel_app_link::desktop::instance::Launch;
    use vessel_app_view::desktop::NavigationTarget;
    if !shell.loaded {
        return;
    }
    let Some(desktop) = cx.try_global::<Desktop>() else {
        return;
    };
    let session = desktop.session;
    let project = desktop.project;
    match shell.areas.desktop.pending.as_ref() {
        Some(Launch::Session(_)) if session.is_none() => return,
        Some(Launch::Project(_)) if project.is_none() => return,
        Some(Launch::Session(_) | Launch::Project(_) | Launch::NewWindow) | None => {}
    }
    match shell.areas.desktop.pending.take() {
        Some(Launch::Session(NavigationTarget::Session { node, session: id })) => {
            if let Some(open) = session {
                open(shell, node, id, window, cx);
            }
        }
        Some(Launch::Project(directory)) => {
            if let Some(open) = project {
                open(shell, directory.path(), window, cx);
            }
        }
        Some(Launch::NewWindow) | None => {}
    }
}

/// The local-node controller installs its asynchronous quit boundary once ready.
pub(crate) fn set_quit_handler(handler: fn(&mut App), cx: &mut App) {
    if cx.has_global::<Desktop>() {
        cx.global_mut::<Desktop>().quit = Some(handler);
    }
}

/// The document owner admits window removal after initiating Shell borrows end.
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

fn install(
    store: Store,
    environment: ShellEnvironment,
    cx: &mut App,
) -> Result<(), vessel_app_link::desktop::DesktopError> {
    cx.set_global(Desktop::new(store, environment)?);
    crate::host_windows::init(cx);
    crate::host_windows::set_factory(windows::open_additional, cx);
    cx.set_quit_mode(gpui::QuitMode::Explicit);
    vessel_app_kit::chrome::set_close_handler(windows::request_close, cx);
    crate::areas::desktop::listen(cx);
    cx.on_window_closed(|cx, _closed| {
        if !cfg!(target_os = "macos") && cx.windows().is_empty() {
            request_quit(cx);
        }
    })
    .detach();
    cx.on_app_quit(|cx| {
        // GPUI cancels observers after 200ms; ordinary quit awaits admission.
        let flush = flush_state(cx);
        async move {
            match flush.await {
                Err(error) => {
                    tracing::warn!(%error,"desktop state could not finish saving before quit");
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

pub(crate) fn with_focused_shell(
    cx: &mut App,
    run: impl FnOnce(&mut Shell, &mut gpui::Window, &mut gpui::Context<'_, Shell>),
) {
    let _delivery = crate::host_windows::with_focused_shell(cx, run);
}

/// App-level complete enumeration; unknown roots refuse rather than disappear.
pub(crate) fn shells(cx: &mut App) -> crate::host_windows::LiveShells {
    crate::host_windows::shells(cx)
}

/// Canonical current local policy is shared across every window.
pub(crate) fn current_local_preferences(cx: &App) -> vessel_app_view::local_node::Preferences {
    cx.global::<Desktop>().preferences.current().local_node
}
