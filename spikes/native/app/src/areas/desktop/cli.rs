//! CLI installation is explicit; status reads and file writes stay off the UI thread.

use crate::shell::Shell;
use gpui::{Context, Window};
use vessel_app_kit::ui::{
    Div, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
};
use vessel_app_kit::{Tokens, outline_button, settings};
use vessel_app_link::desktop::{
    DesktopError,
    bundle::{BundledTool, bundled_tool_path},
    cli::{
        Installation,
        path_setup::{PathSetup, Platform},
    },
};
use vessel_app_view::desktop::cli::{Availability, Operation, PathStatus, Snapshot, State, Status};

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
    let target = home.join(".local/bin").join(if cfg!(windows) {
        "vessel.cmd"
    } else {
        "vessel"
    });
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
    Ok(Snapshot {
        availability,
        status: installation.status()?,
        path,
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
    cx.background_executor().spawn(async move {
        let (installation, path) = prepare()?;
        perform(&installation, &path, operation)
    })
}

fn perform(
    installation: &Installation,
    path: &PathSetup,
    operation: Operation,
) -> Result<Snapshot, DesktopError> {
    let status = match operation {
        Operation::Install => {
            let _installed = installation.install()?;
            match path.ensure() {
                Ok(status) => status,
                Err(error) => {
                    tracing::warn!(%error,"CLI installed; PATH requires manual configuration");
                    PathStatus::Manual
                }
            }
        }
        Operation::Remove => {
            let _removed = installation.uninstall()?;
            path_status(path)
        }
    };
    facts(installation, status)
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
                        shell.areas.desktop.cli.state = State::Failed;
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
        let work = cx
            .background_executor()
            .spawn(async move { perform(&installation, &path, operation) });
        cx.spawn(async move |this, cx| {
            let result = work.await;
            let _closed = this.update(cx, |shell, cx| {
                shell.areas.desktop.cli.state = match result {
                    Ok(state) => state.into(),
                    Err(error) => {
                        tracing::warn!(%error,"CLI operation was refused");
                        State::Failed
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
    .on_click(
        cx.listener(move |shell, _event, _window, cx| shell.desktop_cli_operation(operation, cx)),
    )
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
    .on_click(cx.listener(|shell, _event, _window, cx| shell.desktop_cli_refresh(cx)))
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
