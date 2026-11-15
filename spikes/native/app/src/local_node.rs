//! Process-wide local-node ownership and the asynchronous quit boundary.

use gpui::{App, Global};
use std::rc::Rc;
use vessel_app_link::local_node::{Configuration, Controller, ManagerError};
use vessel_app_view::local_node::{AfterQuit, Management};

pub(crate) struct Managed {
    pub(crate) controller: Option<Rc<Controller>>,
    finalizing: bool,
}
impl Global for Managed {}

pub(crate) fn ensure(prefs: &vessel_app_link::Prefs, cx: &mut App) {
    if cx.has_global::<Managed>() {
        return;
    }
    let controller = match configure(prefs, cx).and_then(Controller::start) {
        Ok(controller) => Some(Rc::new(controller)),
        Err(error) => {
            tracing::warn!(%error, "managed local node could not initialize");
            None
        }
    };
    cx.set_global(Managed {
        controller,
        finalizing: false,
    });
    crate::desktop::set_quit_handler(request_quit, cx);
    // OS shutdown bypasses menu cancellation. Preserve active work by releasing
    // only an owned child if its protected graceful stop is refused. This never
    // sends an unconfirmed StopWhen::Now to a reused or owned node.
    cx.on_app_quit(|cx| {
        let controller = cx
            .try_global::<Managed>()
            .and_then(|managed| managed.controller.clone());
        async move {
            if let Some(controller) = controller {
                if let Err(error) = controller.finalize(AfterQuit::Stop, false).await {
                    tracing::warn!(%error,"protected OS quit preserved the local node");
                    if let Err(error) = controller.finalize(AfterQuit::KeepRunning, false).await {
                        tracing::warn!(%error,"local node release on OS quit failed");
                    }
                }
            }
        }
    })
    .detach();
}

fn configure(prefs: &vessel_app_link::Prefs, cx: &App) -> Result<Configuration, ManagerError> {
    use vessel_app_link::desktop::bundle::{BundledTool, bundled_tool_path};
    use vessel_app_link::desktop::cli::path_setup::Platform;
    let executable = std::env::current_exe().map_err(ManagerError::Runtime)?;
    let platform = if cfg!(windows) {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::Mac
    } else {
        Platform::Unix
    };
    let binary = bundled_tool_path(&executable, platform, BundledTool::Node)
        .map_err(|_unsupported| ManagerError::NotInstalled)?;
    let directory = std::env::var_os("VESSEL_CONFIG_DIR")
        .map(std::path::PathBuf::from)
        .or_else(dirs::config_dir)
        .ok_or(ManagerError::NotInstalled)?;
    let home = directory.join("vessel").join("managed-node");
    let mut preferences = prefs.local_node;
    if matches!(prefs.kept, vessel_app_view::prefs::Kept::ThisLaunch { .. }) {
        preferences.management = Management::Paused;
    }
    Configuration::new(
        binary,
        home,
        &crate::desktop::environment(cx),
        crate::LABEL,
        preferences,
    )
}

pub(crate) fn finalize(
    after: AfterQuit,
    confirmed: bool,
    cx: &App,
) -> impl Future<Output = Result<(), ManagerError>> + 'static {
    let controller = cx
        .try_global::<Managed>()
        .and_then(|managed| managed.controller.clone());
    async move {
        match controller {
            Some(controller) => controller.finalize(after, confirmed).await,
            None => Ok(()),
        }
    }
}

fn request_quit(cx: &mut App) {
    let after = crate::desktop::current_local_preferences(cx).after_quit;
    quit(after, false, cx);
}

pub(crate) fn request_keep_quit(cx: &mut App) {
    quit(AfterQuit::KeepRunning, false, cx);
}
pub(crate) fn request_confirmed_quit(cx: &mut App) {
    quit(AfterQuit::Stop, true, cx);
}

pub(crate) fn begin_finalization(cx: &mut App) -> bool {
    if !cx.has_global::<Managed>() {
        return true;
    }
    let managed = cx.global_mut::<Managed>();
    if managed.finalizing {
        return false;
    }
    managed.finalizing = true;
    true
}

pub(crate) fn end_finalization(cx: &mut App) {
    if cx.has_global::<Managed>() {
        cx.global_mut::<Managed>().finalizing = false;
    }
}

fn quit(after: AfterQuit, confirmed: bool, cx: &mut App) {
    if !begin_finalization(cx) {
        return;
    }
    let finalize = finalize(after, confirmed, cx);
    let flush = crate::desktop::flush_preferences(cx);
    cx.spawn(async move |cx| {
        let result = finalize.await;
        match result {
            Ok(()) => match flush.await {
                Ok(()) => cx.update(App::quit),
                Err(error) => cx.update(|cx| failed(cx, &error)),
            },
            Err(error) => cx.update(|cx| failed(cx, &error)),
        }
    })
    .detach();
}

fn failed(cx: &mut App, error: &dyn std::fmt::Display) {
    tracing::warn!(%error,"quit finalization did not complete; app remains open");
    end_finalization(cx);
    if cx.windows().is_empty() {
        crate::desktop::open_additional(cx);
    }
    crate::desktop::with_focused_shell(cx, |shell, _window, cx| {
        shell.areas.local_node.quit_refused = true;
        shell.route_to(crate::routes::Route::Settings);
        shell.areas.settings.section = vessel_app_view::settings::Section::General;
        shell.notice = Some(shell.words.local_node().quit_refused().to_owned());
        cx.notify();
    });
}
