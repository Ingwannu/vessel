//! Process-wide local-node ownership and the asynchronous quit boundary.

mod documents;
pub(crate) use documents::admit as admit_documents;

use gpui::{App, Global};
use std::rc::Rc;
use vessel_app_link::local_node::{Configuration, Confirmation, Controller, ManagerError};
use vessel_app_view::local_node::{AfterQuit, Management, Status};

pub(crate) struct Managed {
    pub(crate) controller: Option<Rc<Controller>>,
    unavailable: Status,
}
impl Global for Managed {}

pub(crate) fn ensure(prefs: &vessel_app_link::Prefs, cx: &mut App) {
    crate::quit::initialize(cx);
    if cx.has_global::<Managed>() {
        return;
    }
    let (controller, unavailable) = match configure(prefs, cx).and_then(Controller::start) {
        Ok(controller) => (Some(Rc::new(controller)), Status::Working),
        Err(error) => {
            let status = if matches!(error, ManagerError::NotInstalled) {
                Status::NotInstalled
            } else {
                Status::Failed
            };
            tracing::warn!(%error, "managed local node could not initialize");
            (None, status)
        }
    };
    if let Some(controller) = &controller {
        let mut signals = controller.quit_signals();
        cx.spawn(async move |cx| {
            while let Some(signal) = signals.recv().await {
                match signal {
                    Ok(_intent) => cx.update(crate::desktop::request_quit),
                    Err(error) => tracing::warn!(%error,"desktop quit signal observer failed"),
                }
            }
        })
        .detach();
    }
    cx.set_global(Managed {
        controller,
        unavailable,
    });
    crate::desktop::set_quit_handler(request_quit, cx);
    crate::desktop::set_window_close_handler(crate::quit::admit_shell, cx);
    // GPUI gives non-vetoable quit observers only 200ms. Hand off runtime
    // ownership synchronously; a cancelled observer cannot drop an owned node.
    // Ordinary quit/update have already awaited their protected finalizers.
    cx.on_app_quit(|cx| {
        let controller = cx
            .try_global::<Managed>()
            .and_then(|managed| managed.controller.clone());
        if let Some(controller) = controller
            && let Err(error) = controller.preserve_on_exit()
        {
            tracing::warn!(%error,"OS exit runtime handoff was unavailable");
        }
        async {}
    })
    .detach();
}

/// Unsupported/unpublished bundle layouts remain a normal distribution state.
pub(crate) fn unavailable_status(cx: &App) -> Status {
    cx.try_global::<Managed>()
        .map_or(Status::Failed, |managed| managed.unavailable)
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
    confirmation: Option<Confirmation>,
    cx: &App,
) -> impl Future<Output = Result<(), ManagerError>> + 'static + use<> {
    let controller = cx
        .try_global::<Managed>()
        .and_then(|managed| managed.controller.clone());
    async move {
        match controller {
            Some(controller) => controller.finalize(after, confirmation).await,
            None => Ok(()),
        }
    }
}

fn request_quit(cx: &mut App) {
    quit(None, None, cx);
}

pub(crate) fn request_keep_quit(cx: &mut App) {
    quit(Some(AfterQuit::KeepRunning), None, cx);
}
pub(crate) fn request_confirmed_quit(confirmation: Confirmation, cx: &mut App) {
    quit(Some(AfterQuit::Stop), Some(confirmation), cx);
}

pub(crate) fn begin_finalization(cx: &mut App) -> bool {
    crate::quit::begin(cx)
}

pub(crate) fn end_finalization(cx: &mut App) {
    crate::quit::finish(cx);
}

fn quit(after: Option<AfterQuit>, confirmation: Option<Confirmation>, cx: &mut App) {
    if !begin_finalization(cx) {
        return;
    }
    // A window can request quit immediately after its preference edit. Settle
    // the canonical actor first so the quit policy cannot read an older window.
    let before = crate::desktop::flush_preferences(cx);
    cx.spawn(async move |cx| {
        // App-level admission runs after a menu/button's Shell borrow ends.
        if !cx.update(admit_documents) {
            cx.update(end_finalization);
            return;
        }
        if let Err(error) = before.await {
            cx.update(|cx| failed(cx, &error, None));
            return;
        }
        let after = after.unwrap_or_else(|| {
            cx.update(|cx| crate::desktop::current_local_preferences(cx).after_quit)
        });
        let finalize = cx.update(|cx| finalize(after, confirmation, cx));
        let result = finalize.await;
        match result {
            Ok(()) => match cx.update(|cx| crate::desktop::flush_preferences(cx)).await {
                Ok(()) => cx.update(|cx| {
                    if admit_documents(cx) {
                        cx.quit();
                    } else {
                        end_finalization(cx);
                    }
                }),
                Err(error) => cx.update(|cx| failed(cx, &error, None)),
            },
            Err(error) => cx.update(|cx| failed(cx, &error, error.failure_snapshot().cloned())),
        }
    })
    .detach();
}

pub(crate) fn failed(
    cx: &mut App,
    error: &dyn std::fmt::Display,
    snapshot: Option<vessel_app_link::local_node::Snapshot>,
) {
    tracing::warn!(%error,"quit finalization did not complete; app remains open");
    end_finalization(cx);
    if cx.windows().is_empty() {
        crate::desktop::open_additional(cx);
    }
    crate::desktop::with_focused_shell(cx, |shell, _window, cx| {
        shell.local_node_quit_refused(snapshot);
        shell.notice = Some(shell.words.local_node().quit_refused().to_owned());
        cx.notify();
    });
}
