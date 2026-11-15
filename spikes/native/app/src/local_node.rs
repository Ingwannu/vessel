//! Process-wide local-node ownership and the asynchronous quit boundary.

mod documents;
mod state;
pub(crate) use documents::{admit as admit_documents, admit_node};
pub(crate) use state::flush_then;

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
    cx.spawn(async move |cx| {
        // App-level admission runs after a menu/button's Shell borrow ends.
        let settled = flush_then(cx, |cx| {
            if !admit_documents(cx) {
                end_finalization(cx);
                return None;
            }
            Some(after.unwrap_or_else(|| crate::desktop::current_local_preferences(cx).after_quit))
        })
        .await;
        let after = match settled {
            Ok(Some(after)) => after,
            Ok(None) => return,
            Err(error) => {
                cx.update(|cx| failed(cx, &error, Failure::State));
                return;
            }
        };
        // Node finalization is exactly once; Changed state receipts below retry
        // only their persistence phase, with a fresh independent deadline.
        let finalize = cx.update(|cx| finalize(after, confirmation, cx));
        match finalize.await {
            Ok(()) => {
                if let Err(error) = flush_then(cx, |cx| {
                    if admit_documents(cx) {
                        cx.quit();
                    } else {
                        end_finalization(cx);
                    }
                })
                .await
                {
                    cx.update(|cx| failed(cx, &error, Failure::State));
                }
            }
            Err(error) => {
                cx.update(|cx| {
                    failed(cx, &error, Failure::Node(error.failure_snapshot().cloned()))
                });
            }
        }
    })
    .detach();
}

enum Failure {
    State,
    Node(Option<vessel_app_link::local_node::Snapshot>),
}

fn failed(cx: &mut App, error: &dyn std::fmt::Display, failure: Failure) {
    tracing::warn!(%error,"quit finalization did not complete; app remains open");
    end_finalization(cx);
    if cx.windows().is_empty() {
        crate::desktop::open_additional(cx);
    }
    crate::desktop::with_focused_shell(cx, |shell, _window, cx| {
        let message = match failure {
            Failure::State => {
                shell.local_node_quit_refused(None);
                shell.words.quit().state_unsaved().to_owned()
            }
            Failure::Node(snapshot) => {
                shell.local_node_quit_refused(snapshot);
                shell.words.local_node().quit_refused().to_owned()
            }
        };
        shell.notice = Some(message);
        cx.notify();
    });
}
