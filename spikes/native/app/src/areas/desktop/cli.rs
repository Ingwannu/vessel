//! CLI installation is explicit; status reads and file writes stay off the UI thread.

use crate::shell::Shell;
use gpui::{Context, Window};
use vessel_app_kit::ui::{
    Div, FluentBuilder as _, InteractiveElement as _, IntoElement as _, ParentElement as _,
};
use vessel_app_kit::{Tokens, outline_button, settings};
use vessel_app_link::desktop::{
    Error,
    cli::{
        Installation, bundled_cli_path,
        path_setup::{PathSetup, Platform},
    },
};
use vessel_app_view::desktop::cli::{Availability, Operation, PathStatus, State, Status};

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

fn prepare() -> Result<(Installation, PathSetup), Error> {
    let home = dirs::home_dir().ok_or(Error::RelativePath)?;
    let source = bundled_cli_path(&std::env::current_exe()?, platform())?;
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

fn facts(installation: &Installation, path: PathStatus) -> Result<State, Error> {
    let availability = if installation.has_bundled_cli() {
        Availability::Bundled
    } else {
        Availability::Missing
    };
    Ok(State::Ready {
        availability,
        status: installation.status()?,
        path,
    })
}

impl Shell {
    fn desktop_cli_refresh(&mut self, cx: &mut Context<'_, Self>) {
        if matches!(self.areas.desktop.cli.state, State::Working(_)) {
            return;
        }
        self.areas.desktop.cli.state = State::Checking;
        let work = cx.background_executor().spawn(async move {
            let (installation, path) = prepare()?;
            let state = facts(&installation, PathStatus::Manual)?;
            Ok::<_, Error>((installation, path, state))
        });
        cx.spawn(async move |this, cx| {
            let result = work.await;
            let _closed = this.update(cx, |shell, cx| {
                match result {
                    Ok((installation, path, state)) => {
                        shell.areas.desktop.cli.installation = Some(installation);
                        shell.areas.desktop.cli.path = Some(path);
                        shell.areas.desktop.cli.state = state;
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
        let work=cx.background_executor().spawn(async move {
            let path=match operation {
                Operation::Install=>{
                    let _installed=installation.install()?;
                    match path.ensure() {
                        Ok(status)=>status,
                        Err(error)=>{tracing::warn!(%error,"CLI installed; PATH requires manual configuration");PathStatus::Manual}
                    }
                }
                Operation::Remove=>{let _removed=installation.uninstall()?;PathStatus::Manual}
            };
            facts(&installation,path)
        });
        cx.spawn(async move |this, cx| {
            let result = work.await;
            let _closed = this.update(cx, |shell, cx| {
                shell.areas.desktop.cli.state = match result {
                    Ok(state) => state,
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

pub(crate) fn section(shell: &Shell, tokens: &Tokens, cx: &mut Context<'_, Shell>) -> Option<Div> {
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
    let control =
        outline_button(
            "desktop-cli-operation",
            label.to_owned(),
            state.allows(operation),
            tokens,
        )
        .when(state.allows(operation), |control| {
            control.on_click(cx.listener(move |shell, _event, _window, cx| {
                shell.desktop_cli_operation(operation, cx)
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
    Some(settings::section(words.cli().to_owned(), card, tokens))
}
