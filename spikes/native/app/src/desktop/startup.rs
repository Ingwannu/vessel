//! Startup chooses CLI or one GUI owner before initializing any renderer.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;
use tokio::sync::mpsc;
use vessel_app_kit::SchemeChoice;
use vessel_app_link::desktop::{
    DesktopError,
    environment::{self, LoginShell, ShellEnvironment},
    instance::{self, Instance, Launch},
    invocation::{self, Invocation, Mode},
    store::Store,
};

const OS_LAUNCH_CAPACITY: usize = 32;

pub(crate) fn run(choice: SchemeChoice, application: fn() -> gpui::Application) -> ExitCode {
    match start(choice, application) {
        Ok(code) => code,
        Err(error) => {
            let kind = if matches!(&error, DesktopError::RestartDeadline) {
                super::startup_error::Kind::Restart
            } else {
                super::startup_error::Kind::Launch
            };
            super::startup_error::show(kind, choice, application);
            tracing::error!(%error,"desktop startup could not complete");
            ExitCode::FAILURE
        }
    }
}

fn start(
    choice: SchemeChoice,
    application: fn() -> gpui::Application,
) -> Result<ExitCode, DesktopError> {
    let mode = if std::env::var_os("VESSEL_DESKTOP_CLI").is_some_and(|value| value == "1") {
        Mode::Cli
    } else {
        Mode::Automatic
    };
    let launch = match invocation::parse(std::env::args_os().skip(1), mode)? {
        Invocation::Cli(arguments) => {
            return Ok(match passthrough(arguments) {
                Ok(code) => code,
                Err(error) => {
                    tracing::error!(%error,"Vessel CLI could not start");
                    ExitCode::FAILURE
                }
            });
        }
        Invocation::Gui(launch) => launch,
    };
    let directory = std::env::var_os("VESSEL_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(dirs::config_dir)
        .ok_or(DesktopError::RelativePath)?
        .join("vessel")
        .join("desktop");
    let store = Store::new(directory)?;
    let instance = if std::env::var_os(vessel_app_updater::RESTARTED_FROM).is_some() {
        instance::claim_restart(store.directory(), &launch)?
    } else {
        instance::claim(store.directory(), &launch)?
    };
    let Instance::Primary {
        guard,
        mut launches,
    } = instance
    else {
        return Ok(ExitCode::SUCCESS);
    };
    let environment = resolve_environment();
    let (sender, mut requests) = mpsc::channel(OS_LAUNCH_CAPACITY);
    let application = application().with_assets(vessel_app_kit::Assets);
    let _urls = application.on_open_urls(move |urls| {
        for url in urls {
            match vessel_app_view::desktop::NavigationTarget::parse(&url) {
                Ok(target) => {
                    if let Err(error) = sender.try_send(Launch::Session(target)) {
                        tracing::warn!(%error,"desktop link queue could not accept a request");
                    }
                }
                Err(error) => tracing::warn!(%error,"desktop URL was refused"),
            }
        }
    });
    let _reopen = application.on_reopen(|cx| {
        if cx.windows().is_empty() {
            super::windows::open_primary(None, cx);
        }
    });
    application.run(move |cx| {
        if let Err(error)=vessel_app_kit::init(cx,choice) {tracing::error!(%error,"the kit could not start");cx.quit();return;}
        crate::motion::follow(cx);
        if let Err(error)=super::install(store,environment,cx) {tracing::error!(%error,"desktop state could not start");cx.quit();return;}
        super::windows::open_primary(Some(launch),cx);
        cx.spawn(async move |cx| {
            loop {
                tokio::select! {
                    Some(request)=requests.recv()=>cx.update(|cx|super::windows::deliver(request,cx)),
                    Some(request)=launches.recv()=>cx.update(|cx|super::windows::deliver(request,cx)),
                    else=>break,
                }
            }
        }).detach();
    });
    drop(guard);
    Ok(ExitCode::SUCCESS)
}

fn passthrough(arguments: Vec<OsString>) -> Result<ExitCode, DesktopError> {
    let platform = if cfg!(windows) {
        vessel_app_link::desktop::cli::path_setup::Platform::Windows
    } else if cfg!(target_os = "macos") {
        vessel_app_link::desktop::cli::path_setup::Platform::Mac
    } else {
        vessel_app_link::desktop::cli::path_setup::Platform::Unix
    };
    let binary = vessel_app_link::desktop::bundle::bundled_tool_path(
        &std::env::current_exe()?,
        platform,
        vessel_app_link::desktop::bundle::BundledTool::Cli,
    )?;
    if !binary.is_file() {
        return Err(DesktopError::MissingCli);
    }
    let status = std::process::Command::new(binary)
        .args(arguments)
        .status()?;
    Ok(status
        .code()
        .and_then(|code| u8::try_from(code).ok())
        .map_or(ExitCode::FAILURE, ExitCode::from))
}

fn resolve_environment() -> ShellEnvironment {
    if cfg!(windows) || std::env::var_os("VESSEL_APP_NO_SHELL_ENV").is_some() {
        return ShellEnvironment::inherited();
    }
    let path = std::env::var_os("SHELL").map_or_else(
        || {
            PathBuf::from(if cfg!(target_os = "macos") {
                "/bin/zsh"
            } else {
                "/bin/bash"
            })
        },
        PathBuf::from,
    );
    let result = (|| {
        let shell = LoginShell::new(path)?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        runtime.block_on(environment::resolve(shell, environment::DEFAULT_TIMEOUT))
    })();
    match result {
        Ok(environment) => environment,
        Err(error) => {
            tracing::warn!(%error,"keeping the inherited environment after login-shell resolution failed");
            ShellEnvironment::inherited()
        }
    }
}
