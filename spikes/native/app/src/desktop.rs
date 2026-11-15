//! Process-wide desktop ownership; screens use typed host adapters only.

pub(crate) mod preferences;
mod startup;
mod startup_error;
mod windows;

use crate::shell::Shell;
use gpui::{AnyWindowHandle, App, Global, WeakEntity};
use vessel_app_link::desktop::{
    environment::ShellEnvironment, preferences::Coordinator, store::Store,
    window_writer::WindowWriter,
};
use vessel_app_view::desktop::window::WindowState;

pub(crate) use startup::run;
pub(crate) use windows::open_additional;

#[derive(Clone)]
struct WindowRecord {
    window: AnyWindowHandle,
    shell: WeakEntity<Shell>,
}

pub(crate) struct Desktop {
    records: Vec<WindowRecord>,
    saved: Option<WindowState>,
    latest: WindowState,
    writer: Option<WindowWriter>,
    environment: ShellEnvironment,
    quit: Option<fn(&mut App)>,
    session: Option<SessionHandler>,
    project: Option<ProjectHandler>,
    preferences: Coordinator,
}
impl Global for Desktop {}

impl Desktop {
    fn new(
        store: Store,
        environment: ShellEnvironment,
    ) -> Result<Self, vessel_app_link::desktop::Error> {
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
            writer,
            environment,
            quit: None,
            session: None,
            project: None,
            preferences,
        })
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
) -> Result<(), vessel_app_link::desktop::Error> {
    cx.set_global(Desktop::new(store, environment)?);
    cx.set_quit_mode(gpui::QuitMode::Explicit);
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
        let desktop = cx.global::<Desktop>();
        let flush = flush_preferences(cx);
        let final_write = desktop
            .writer
            .as_ref()
            .map(|writer| writer.finalize(desktop.latest));
        async move {
            if let Err(error) = flush.await {
                tracing::warn!(%error,"preferences could not finish saving before quit");
            }
            if let Some(final_write) = final_write
                && let Err(error) = final_write.await
            {
                tracing::warn!(%error,"final window state could not be saved");
            }
        }
    })
    .detach();
    Ok(())
}

/// Update restart and normal quit await the same canonical preferences queue.
pub(crate) fn flush_preferences(
    cx: &App,
) -> impl Future<Output = Result<(), vessel_app_link::desktop::Error>> + 'static {
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
    let active = cx.active_window();
    let Some(desktop) = cx.try_global::<Desktop>() else {
        return;
    };
    let record = desktop
        .records
        .iter()
        .find(|record| Some(record.window) == active)
        .or_else(|| desktop.records.first())
        .cloned();
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
