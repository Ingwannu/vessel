//! Startup chooses CLI or one GUI owner before initializing any renderer.

use gpui::App;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;
use tokio::sync::mpsc;
use vessel_app_kit::SchemeChoice;
use vessel_app_link::desktop::{
    Error,
    environment::{self, LoginShell, ShellEnvironment},
    instance::{self, Instance, Launch},
    invocation::{self, Invocation, Mode},
    store::Store,
};

const OS_LAUNCH_CAPACITY: usize = 32;

pub(crate) fn run(choice: SchemeChoice, application: fn() -> gpui::Application) -> ExitCode {
    match start(choice, application) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(%error,"desktop startup could not complete");
            ExitCode::FAILURE
        }
    }
}

fn start(choice: SchemeChoice, application: fn() -> gpui::Application) -> Result<(), Error> {
    let mode = if std::env::var_os("VESSEL_DESKTOP_CLI").is_some_and(|value| value == "1") {
        Mode::Cli
    } else {
        Mode::Automatic
    };
    let launch = match invocation::parse(std::env::args_os().skip(1), mode)? {
        Invocation::Cli(arguments) => return passthrough(arguments),
        Invocation::Gui(launch) => launch,
    };
    let directory = std::env::var_os("VESSEL_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(dirs::config_dir)
        .ok_or(Error::RelativePath)?
        .join("vessel")
        .join("desktop");
    let store = Store::new(directory)?;
    let Instance::Primary {
        guard,
        mut launches,
    } = instance::claim(store.directory(), &launch)?
    else {
        return Ok(());
    };
    let environment = resolve_environment();
    let (sender, mut requests) = mpsc::channel(OS_LAUNCH_CAPACITY);
    let application = application().with_assets(vessel_app_kit::Assets);
    application.on_open_urls(move |urls| {
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
    application.on_reopen(|cx| {
        if cx.windows().is_empty() {
            super::windows::open_primary(None, cx);
        }
    });
    application.run(move |cx| {
        if let Err(error)=vessel_app_kit::init(cx,choice) {tracing::error!(%error,"the kit could not start");cx.quit();return;}
        crate::motion::follow(cx);
        super::install(store,environment,cx);
        super::windows::open_primary(Some(launch),cx);
        cx.spawn(async move |cx| {
            loop {
                let next=tokio::select! {request=requests.recv()=>request,request=launches.recv()=>request};
                let Some(request)=next else {break;};
                cx.update(|cx|super::windows::deliver(request,cx));
            }
        }).detach();
    });
    drop(guard);
    Ok(())
}

fn passthrough(arguments: Vec<OsString>) -> Result<(), Error> {
    let binary = std::env::current_exe()?.with_file_name(if cfg!(windows) {
        "vessel.exe"
    } else {
        "vessel"
    });
    if !binary.is_file() {
        return Err(Error::MissingCli);
    }
    let status = std::process::Command::new(binary)
        .args(arguments)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::InvalidLaunch)
    }
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
