//! The updater alone, against the feed compiled in: what the app's update
//! does, without the app, so it can run where the app cannot draw (a
//! Windows build under wine, whose DirectWrite stops the app at its first
//! text). Not shipped; built by hand to test a feed.
//!
//!     probe <executable to update>
//!
//! Prints each state it passes through and ends when the update is in
//! place, the feed has nothing newer, or a check fails. The feed's address
//! is never printed (it carries a token).

use std::{io::Write as _, path::PathBuf, time::Duration};

use vessel_app_updater::{Installation, Packaging, UpdateState, Updater, settings_from_build};

fn say(line: &str) {
    let mut out = std::io::stdout().lock();
    let _ignored = writeln!(out, "{line}");
}

fn main() {
    let Some(target) = std::env::args_os().nth(1).map(PathBuf::from) else {
        say("usage: probe <executable to update>");
        return;
    };
    let mut settings = match settings_from_build() {
        Ok(Some(settings)) => settings,
        Ok(None) => {
            say("this build has no feed compiled in");
            return;
        }
        Err(error) => {
            say(&format!("the build's settings do not parse: {error}"));
            return;
        }
    };
    settings.first_after = Duration::ZERO;
    say(&format!(
        "running {} on {}, channel {}",
        settings.running, settings.target, settings.channel
    ));
    let installation = match Installation::detect(Packaging::Executable, target, None) {
        Ok(installation) => installation,
        Err(error) => {
            say(&format!("no installation: {error}"));
            return;
        }
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            say(&format!("no runtime: {error}"));
            return;
        }
    };
    let updater = match Updater::start(settings, installation, runtime.handle()) {
        Ok(updater) => updater,
        Err(error) => {
            say(&format!("the updater did not start: {error}"));
            return;
        }
    };
    let mut state = updater.state();
    runtime.block_on(async {
        let mut last_tenth = None;
        loop {
            let now = state.borrow_and_update().clone();
            match &now {
                UpdateState::Downloading {
                    label,
                    received,
                    total,
                } => {
                    let tenth = received
                        .get()
                        .saturating_mul(10)
                        .checked_div(total.get().max(1));
                    if tenth != last_tenth {
                        last_tenth = tenth;
                        say(&format!(
                            "downloading {label}: {} of {} bytes",
                            received.get(),
                            total.get()
                        ));
                    }
                }
                UpdateState::Idle | UpdateState::Checking => say(&format!("{now:?}")),
                UpdateState::UpToDate { latest } => {
                    say(&format!("up to date: the feed's newest is {latest}"));
                    return;
                }
                UpdateState::Ready { label, .. } => {
                    say(&format!("ready: {label} is in place"));
                    return;
                }
                UpdateState::Failed { error, .. } => {
                    say(&format!("failed: {error}"));
                    return;
                }
            }
            if state.changed().await.is_err() {
                return;
            }
        }
    });
    drop(updater);
    runtime.shutdown_background();
}
