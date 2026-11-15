//! Explicit no-home/no-node process owner for desktop behavior fixtures.

use super::Desktop;
use vessel_app_link::desktop::{environment::ShellEnvironment, preferences::Coordinator};
use vessel_app_view::desktop::window::WindowState;

pub(super) fn desktop(preferences: Coordinator) -> Desktop {
    Desktop {
        records: Vec::new(),
        saved: None,
        latest: WindowState::default(),
        writer: None,
        environment: ShellEnvironment::inherited(),
        environment_broadcast: None,
        quit: None,
        window_close: None,
        session: None,
        project: None,
        preferences,
    }
}
