//! GUI launch routing is decided before generic CLI argument passthrough.

use super::{
    DesktopError,
    instance::{Launch, ProjectDirectory},
};
use std::ffi::OsString;
use std::path::PathBuf;
use vessel_app_view::desktop::NavigationTarget;

/// Explicit CLI forcing is separate from incidental platform launch flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Recognize OS links and project launches before command passthrough.
    Automatic,
    /// Even an empty argument list belongs to the CLI.
    Cli,
}

/// Only one startup path is selected, before an app or daemon is initialized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invocation {
    /// A GUI request may be forwarded to the running process.
    Gui(Launch),
    /// Arguments are passed verbatim to the bundled CLI.
    Cli(Vec<OsString>),
}

/// Ignore only the platform flags Paseo ignores; reject malformed GUI links.
///
/// # Errors
/// Invalid GUI paths/links and incomplete project flags are explicit refusals.
pub fn parse(
    arguments: impl IntoIterator<Item = OsString>,
    mode: Mode,
) -> Result<Invocation, DesktopError> {
    let arguments: Vec<_> = arguments
        .into_iter()
        .filter(|argument| {
            let Some(text) = argument.to_str() else {
                return true;
            };
            ![
                "-psn_",
                "--class=",
                "--no-sandbox",
                "--remote-debugging-port=",
            ]
            .iter()
            .any(|prefix| text.starts_with(prefix))
        })
        .collect();
    if mode == Mode::Cli {
        return Ok(Invocation::Cli(arguments));
    }
    if let Some(argument) = arguments.iter().find(|arg| {
        arg.to_str().is_some_and(|text| {
            text.split_once(':')
                .is_some_and(|(scheme, _)| scheme.eq_ignore_ascii_case("vessel"))
        })
    }) {
        // OS scheme matching is case-insensitive. Even a noncanonical URI
        // must reach the strict navigation parser, never generic CLI commands.
        let text = argument.to_str().ok_or(DesktopError::InvalidLaunch)?;
        return NavigationTarget::parse(text)
            .map(|target| Invocation::Gui(Launch::Session(target)))
            .map_err(|_invalid| DesktopError::InvalidLaunch);
    }
    match arguments.as_slice() {
        [] => Ok(Invocation::Gui(Launch::NewWindow)),
        [flag, path] if flag == "--open-project" => ProjectDirectory::new(PathBuf::from(path))
            .map(|dir| Invocation::Gui(Launch::Project(dir))),
        [flag] if flag == "--open-project" => Err(DesktopError::InvalidLaunch),
        [path] if PathBuf::from(path).is_absolute() => ProjectDirectory::new(PathBuf::from(path))
            .map(|dir| Invocation::Gui(Launch::Project(dir))),
        _ => Ok(Invocation::Cli(arguments)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_targets_win_over_passthrough_without_leaking_malformed_links() {
        use vessel_proto::{NodeId, SessionId};
        let target = NavigationTarget::Session {
            node: NodeId::random(),
            session: SessionId::random(),
        };
        assert_eq!(
            parse([target.to_url().into()], Mode::Automatic).unwrap(),
            Invocation::Gui(Launch::Session(target))
        );
        assert!(
            parse(
                ["vessel://h/invalid?credential=not-retained".into()],
                Mode::Automatic
            )
            .is_err()
        );
        assert!(parse(["--open-project".into()], Mode::Automatic).is_err());
        assert_eq!(
            parse(["--class=Vessel".into()], Mode::Automatic).unwrap(),
            Invocation::Gui(Launch::NewWindow)
        );
        assert_eq!(
            parse(["status".into()], Mode::Automatic).unwrap(),
            Invocation::Cli(vec!["status".into()])
        );
        assert_eq!(parse([], Mode::Cli).unwrap(), Invocation::Cli(Vec::new()));
    }

    #[test]
    fn os_scheme_case_variants_and_extra_arguments_never_fall_into_cli_execution() {
        use vessel_proto::{NodeId, SessionId};
        let target = NavigationTarget::Session {
            node: NodeId::random(),
            session: SessionId::random(),
        };
        assert_eq!(
            parse(
                [target.to_url().into(), "status".into(), "--json".into()],
                Mode::Automatic
            )
            .unwrap(),
            Invocation::Gui(Launch::Session(target))
        );
        for scheme in ["VESSEL", "Vessel", "vEsSeL"] {
            assert!(matches!(
                parse(
                    [format!("{scheme}://h/refused").into(), "status".into()],
                    Mode::Automatic
                ),
                Err(DesktopError::InvalidLaunch)
            ));
        }
    }
}
