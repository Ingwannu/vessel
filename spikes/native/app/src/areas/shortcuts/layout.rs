//! The one keyboard dispatcher delegates local pane operations to W12. Resource
//! launch/teardown remains with feature controllers; guarded close is handled.
//! New terminal (Paseo's `workspace-terminal-new`) goes to the terminal tabs'
//! owner, as the "+" menu's Terminal does.
use super::dispatch::{Callbacks, KeyboardHandler, Registration, ShortcutRequest};
use crate::{routes::Route, shell::Shell};
use vessel_app_kit::ui::{App, Context, Window};
use vessel_app_view::shortcuts::dispatcher::{DispatchError, Handled, Priority};
use vessel_app_view::shortcuts::{Action, context::Admission, engine::ShortcutIndex};

const FIRST_TAB: u8 = 1;
const LAYOUT_PRIORITY: i16 = 100;

pub(super) fn register(shell: &mut Shell) -> Result<Registration, DispatchError> {
    shell.register_shortcut_handler(KeyboardHandler {
        id: "native-layout".parse()?,
        actions: Action::ALL
            .into_iter()
            .filter(|action| {
                *action == Action::NewTerminal
                    || action
                        .layout_action(ShortcutIndex::new(FIRST_TAB))
                        .is_some()
            })
            .collect(),
        priority: Priority::new(LAYOUT_PRIORITY),
        callbacks: Callbacks { enabled, perform },
    })
}
fn enabled(shell: &Shell, request: &ShortcutRequest, _window: &Window, _cx: &App) -> bool {
    request.context.admission == Admission::Available
        && shell.route == Route::Session
        && shell.areas.session.open.is_some()
        && (request.action == Action::NewTerminal
            || request.action.layout_action(request.index).is_some())
}
fn perform(
    shell: &mut Shell,
    request: ShortcutRequest,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> Handled {
    if request.action == Action::NewTerminal {
        shell.terminal_tab_new(None, window, cx);
        return Handled::Yes;
    }
    let Some(action) = request.action.layout_action(request.index) else {
        return Handled::No;
    };
    if shell.run_layout_action(action, window, cx) {
        Handled::Yes
    } else {
        Handled::No
    }
}
