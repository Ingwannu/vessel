//! CLI installation is explicit; status reads and file writes stay off the UI thread.

use crate::shell::Shell;
use gpui::{Context, Window};
use vessel_app_kit::ui::{
    Div, FluentBuilder as _, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
};
use vessel_app_kit::{Tokens, outline_button, settings};
use vessel_app_link::desktop::{
    DesktopError,
    bundle::{BundledTool, bundled_tool_path},
    cli::{
        Change, Installation,
        path_setup::{PathSetup, Platform},
    },
};
use vessel_app_view::desktop::cli::{Availability, Operation, PathStatus, Snapshot, State, Status};

// Multiple windows and deferred onboarding use this same process operation.
// Refuse overlapping writes instead of blocking a UI/background executor thread.
static MUTATION: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(crate) struct CliPanel {
    state: State,
    installation: Option<Installation>,
    path: Option<PathSetup>,
}

impl Default for CliPanel {
    fn default() -> Self {
        Self {
            state: State::Checking,
            installation: None,
            path: None,
        }
    }
}

pub(crate) fn start(shell: &mut Shell, _window: &mut Window, cx: &mut Context<'_, Shell>) {
    shell.desktop_cli_refresh(cx);
}

fn platform() -> Platform {
    if cfg!(windows) {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::Mac
    } else {
        Platform::Unix
    }
}

fn prepare() -> Result<(Installation, PathSetup), DesktopError> {
    let home = dirs::home_dir().ok_or(DesktopError::RelativePath)?;
    let source = bundled_tool_path(&std::env::current_exe()?, platform(), BundledTool::Cli)?;
    let target = if cfg!(windows) {
        source
            .parent()
            .ok_or(DesktopError::RelativePath)?
            .to_path_buf()
    } else {
        home.join(".local/bin/vessel")
    };
    let shell = std::env::var_os("SHELL").map(std::path::PathBuf::from);
    let path = PathSetup::new(
        &home,
        shell.as_deref(),
        platform(),
        std::env::var_os("PATH").unwrap_or_default(),
    )?;
    Ok((Installation::new(source, target)?, path))
}

fn facts(installation: &Installation, path: PathStatus) -> Result<Snapshot, DesktopError> {
    let availability = if installation.has_bundled_cli() {
        Availability::Bundled
    } else {
        Availability::Missing
    };
    let status = installation.status()?;
    Ok(Snapshot {
        availability,
        status,
        path: if cfg!(windows) && status == Status::Installed {
            PathStatus::Configured
        } else {
            path
        },
    })
}

fn path_status(path: &PathSetup) -> PathStatus {
    match path.status() {
        Ok(status) => status,
        Err(error) => {
            tracing::warn!(%error,"CLI PATH status needs manual inspection");
            PathStatus::Manual
        }
    }
}

fn probe() -> Result<(Installation, PathSetup, Snapshot), DesktopError> {
    let (installation, path) = prepare()?;
    let snapshot = facts(&installation, path_status(&path))?;
    Ok((installation, path, snapshot))
}

/// Onboarding reads offered/current facts without applying its default choice.
pub(crate) fn snapshot(cx: &gpui::App) -> gpui::Task<Result<Snapshot, DesktopError>> {
    cx.background_executor()
        .spawn(async { probe().map(|(_installation, _path, snapshot)| snapshot) })
}

/// Capture work only on explicit Continue; the acknowledgement includes PATH hints.
pub(crate) fn set_installed(
    operation: Operation,
    cx: &gpui::App,
) -> gpui::Task<Result<Snapshot, DesktopError>> {
    let broadcast = crate::desktop::environment_broadcast(cx);
    let executor = cx.background_executor().clone();
    cx.background_executor().spawn(async move {
        let (installation, path) = prepare()?;
        let applied = perform(&installation, &path, operation)?;
        Ok(finish(applied, broadcast, &executor))
    })
}

struct Applied {
    snapshot: Snapshot,
    environment: Change,
}

fn perform(
    installation: &Installation,
    path: &PathSetup,
    operation: Operation,
) -> Result<Applied, DesktopError> {
    let _permit = match MUTATION.try_lock() {
        Ok(permit) => permit,
        Err(std::sync::TryLockError::WouldBlock) => return Err(DesktopError::CliOperationBusy),
        // Every attempt rechecks exact ownership and current PATH/rc facts;
        // a previous unwind cannot authorize overwriting foreign content.
        Err(std::sync::TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
    };
    let mutation = installation.mutate(operation)?;
    let (status, path_change) = match operation {
        Operation::Install => match path.ensure_change() {
            Ok(changed) => (changed.status, changed.change),
            Err(error) => {
                tracing::warn!(%error,"CLI installed; PATH requires manual configuration");
                (PathStatus::Manual, Change::Unchanged)
            }
        },
        Operation::Remove => (path_status(path), Change::Unchanged),
    };
    let environment = if cfg!(windows) {
        mutation.change
    } else {
        path_change
    };
    Ok(Applied {
        snapshot: facts(installation, status)?,
        environment,
    })
}

fn finish(
    applied: Applied,
    broadcast: Option<crate::desktop::EnvironmentBroadcast>,
    executor: &gpui::BackgroundExecutor,
) -> Snapshot {
    if applied.environment == Change::Changed
        && let Some(broadcast) = broadcast
    {
        executor.spawn(async move {
            if let Err(error) = broadcast() {
                tracing::warn!(%error,"CLI PATH changed; environment notification needs a fresh login");
            }
        }).detach();
    }
    // Snapshot reports the completed mutation immediately. Configured guidance
    // always includes a fresh terminal/login; notification is separate advice.
    applied.snapshot
}

fn failed(error: &DesktopError) -> State {
    if matches!(error, DesktopError::UserPathTransaction(_)) {
        State::UserPathUnavailable
    } else {
        State::Failed
    }
}

impl Shell {
    fn desktop_cli_refresh(&mut self, cx: &mut Context<'_, Self>) {
        if matches!(self.areas.desktop.cli.state, State::Working(_)) {
            return;
        }
        self.areas.desktop.cli.state = State::Checking;
        let work = cx.background_executor().spawn(async move { probe() });
        cx.spawn(async move |this, cx| {
            let result = work.await;
            let _closed = this.update(cx, |shell, cx| {
                match result {
                    Ok((installation, path, state)) => {
                        shell.areas.desktop.cli.installation = Some(installation);
                        shell.areas.desktop.cli.path = Some(path);
                        shell.areas.desktop.cli.state = state.into();
                    }
                    Err(error) => {
                        tracing::warn!(%error,"CLI status could not be read");
                        shell.areas.desktop.cli.state = failed(&error);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn desktop_cli_operation(&mut self, operation: Operation, cx: &mut Context<'_, Self>) {
        let panel = &mut self.areas.desktop.cli;
        if !panel.state.allows(operation) {
            return;
        }
        let (Some(installation), Some(path)) = (panel.installation.clone(), panel.path.clone())
        else {
            panel.state = State::Failed;
            cx.notify();
            return;
        };
        panel.state = State::Working(operation);
        cx.notify();
        let broadcast = crate::desktop::environment_broadcast(cx);
        let executor = cx.background_executor().clone();
        let work = cx.background_executor().spawn(async move {
            let applied = perform(&installation, &path, operation)?;
            Ok(finish(applied, broadcast, &executor))
        });
        cx.spawn(async move |this, cx| {
            let result = work.await;
            let _closed = this.update(cx, |shell, cx| {
                shell.areas.desktop.cli.state = match result {
                    Ok(state) => state.into(),
                    Err(error) => {
                        tracing::warn!(%error,"CLI operation was refused");
                        failed(&error)
                    }
                };
                cx.notify();
            });
        })
        .detach();
    }
}

pub(crate) fn section(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Div {
    let words = shell.words.desktop_shell();
    let state = shell.areas.desktop.cli.state;
    let hint = match state {
        State::Checking => words.cli_checking(),
        State::Working(_) => words.cli_busy(),
        State::Failed => words.cli_failed(),
        State::UserPathUnavailable => words.cli_path_transaction_failed(),
        State::Ready {
            status: Status::Foreign,
            ..
        } => words.cli_foreign(),
        State::Ready {
            availability: Availability::Missing,
            ..
        } => words.cli_missing(),
        State::Ready {
            availability: Availability::Bundled,
            status: Status::Installed,
            ..
        } => words.cli_installed(),
        State::Ready {
            availability: Availability::Bundled,
            status: Status::Absent,
            ..
        } => words.cli_absent(),
    };
    let operation = if matches!(
        state,
        State::Ready {
            status: Status::Installed,
            ..
        }
    ) {
        Operation::Remove
    } else {
        Operation::Install
    };
    let label = match operation {
        Operation::Install => words.install_cli(),
        Operation::Remove => words.remove_cli(),
    };
    let control = outline_button(
        "desktop-cli-operation",
        label.to_owned(),
        state.allows(operation),
        tokens,
    )
    .when(state.allows(operation), |control| {
        control.on_click(cx.listener(move |shell, _event, _window, cx| {
            shell.desktop_cli_operation(operation, cx);
        }))
    })
    .into_any_element();
    let mut card = settings::card(tokens).child(settings::row(
        words.cli(),
        Some(hint.to_owned().into()),
        Some(control),
        true,
        tokens,
    ));
    if let State::Ready {
        status: Status::Installed,
        path,
        ..
    } = state
    {
        let hint = match path {
            PathStatus::Available => words.cli_installed(),
            PathStatus::Configured => words.path_ready(),
            PathStatus::Manual => words.path_manual(),
        };
        card = card.child(settings::row(
            words.cli_path(),
            Some(hint.to_owned().into()),
            None,
            false,
            tokens,
        ));
    }
    let refresh = outline_button(
        "desktop-cli-refresh",
        words.refresh().to_owned(),
        !matches!(state, State::Working(_)),
        tokens,
    )
    .when(!matches!(state, State::Working(_)), |control| {
        control.on_click(cx.listener(|shell, _event, _window, cx| shell.desktop_cli_refresh(cx)))
    })
    .into_any_element();
    card = card.child(settings::row(
        words.refresh(),
        None,
        Some(refresh),
        false,
        tokens,
    ));
    settings::section(words.cli().to_owned(), card, tokens)
}

#[cfg(all(test, unix))]
mod broadcast_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NOTIFICATIONS: AtomicUsize = AtomicUsize::new(0);
    struct Home(std::path::PathBuf);
    impl Drop for Home {
        fn drop(&mut self) {
            let _cleanup = std::fs::remove_dir_all(&self.0);
        }
    }
    fn timed_out_notification() -> Result<(), std::io::Error> {
        let _previous = NOTIFICATIONS.fetch_add(1, Ordering::SeqCst);
        Err(std::io::ErrorKind::TimedOut.into())
    }
    #[gpui::test]
    fn snapshot_does_not_wait_for_notification_and_unchanged_path_does_not_notify(
        cx: &mut gpui::TestAppContext,
    ) {
        NOTIFICATIONS.store(0, Ordering::SeqCst);
        let home = Home(std::env::temp_dir().join(format!(
            "vessel-cli-broadcast-{}",
            vessel_proto::SessionId::random()
        )));
        std::fs::create_dir_all(&home.0).unwrap();
        let source = home.0.join("bundled-cli");
        let installation =
            Installation::new(source.clone(), home.0.join(".local/bin/vessel")).unwrap();
        let path = PathSetup::new(
            &home.0,
            Some(std::path::Path::new("/bin/bash")),
            Platform::Unix,
            std::ffi::OsString::new(),
        )
        .unwrap();
        assert!(matches!(
            perform(&installation, &path, Operation::Install),
            Err(DesktopError::MissingCli)
        ));
        std::fs::write(source, b"nonexecuted CLI fixture").unwrap();
        let permit = MUTATION.try_lock().unwrap();
        assert!(matches!(
            perform(&installation, &path, Operation::Install),
            Err(DesktopError::CliOperationBusy)
        ));
        assert_eq!(installation.status().unwrap(), Status::Absent);
        assert!(!home.0.join(".bashrc").exists());
        drop(permit);
        let applied = perform(&installation, &path, Operation::Install).unwrap();
        assert_eq!(applied.environment, Change::Changed);
        let snapshot = finish(
            applied,
            Some(timed_out_notification),
            &cx.background_executor,
        );
        assert_eq!(snapshot.status, Status::Installed);
        assert_eq!(NOTIFICATIONS.load(Ordering::SeqCst), 0);
        cx.run_until_parked();
        assert_eq!(NOTIFICATIONS.load(Ordering::SeqCst), 1);
        assert_eq!(installation.status().unwrap(), Status::Installed);
        let unchanged = perform(&installation, &path, Operation::Install).unwrap();
        assert_eq!(unchanged.environment, Change::Unchanged);
        let _snapshot = finish(
            unchanged,
            Some(timed_out_notification),
            &cx.background_executor,
        );
        cx.run_until_parked();
        assert_eq!(NOTIFICATIONS.load(Ordering::SeqCst), 1);
    }
}
