//! Desktop menus and pending OS intents; authenticated destinations stay with owners.

#[cfg(not(target_arch = "wasm32"))]
mod actions;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod cli;
#[cfg(not(target_arch = "wasm32"))]
mod menus;
#[cfg(not(target_arch = "wasm32"))]
mod native;
pub(crate) mod preferences;

use crate::shell::Shell;
#[cfg(target_arch = "wasm32")]
use vessel_app_kit::ui::AnyElement;
use vessel_app_kit::ui::{Context, Window};

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use menus::{render, trigger};

/// Per-window UI state does not own process-wide lifecycle resources.
pub(crate) struct DesktopArea {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) pending: Option<vessel_app_link::desktop::instance::Launch>,
    #[cfg(not(target_arch = "wasm32"))]
    menu: vessel_app_kit::menu::MenuState,
    #[cfg(not(target_arch = "wasm32"))]
    group: menus::Group,
    #[cfg(not(target_arch = "wasm32"))]
    focus: Option<gpui::FocusHandle>,
    #[cfg(not(target_arch = "wasm32"))]
    zoom: actions::Zoom,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) preferences: Option<vessel_app_link::desktop::preferences::Client>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) rollback: Option<vessel_app_link::Prefs>,
    #[cfg(not(target_arch = "wasm32"))]
    cli: cli::CliPanel,
}

impl DesktopArea {
    pub(crate) fn new(cx: &mut gpui::App) -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            pending: None,
            #[cfg(not(target_arch = "wasm32"))]
            menu: vessel_app_kit::menu::MenuState::new(cx),
            #[cfg(not(target_arch = "wasm32"))]
            group: menus::Group::File,
            #[cfg(not(target_arch = "wasm32"))]
            focus: None,
            #[cfg(not(target_arch = "wasm32"))]
            zoom: actions::Zoom::default(),
            #[cfg(not(target_arch = "wasm32"))]
            preferences: None,
            #[cfg(not(target_arch = "wasm32"))]
            rollback: None,
            #[cfg(not(target_arch = "wasm32"))]
            cli: cli::CliPanel::default(),
        }
    }
}

pub(crate) fn pending(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    preferences::rollback(shell, window, cx);
    #[cfg(not(target_arch = "wasm32"))]
    crate::desktop::drain_pending(shell, window, cx);
    #[cfg(target_arch = "wasm32")]
    let _browser = (shell, window, cx);
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn trigger(
    _shell: &Shell,
    _tokens: &vessel_app_kit::Tokens,
    _window: &Window,
    _cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    None
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn render(
    _shell: &Shell,
    _tokens: &vessel_app_kit::Tokens,
    _window: &Window,
    _cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    None
}

pub(crate) fn start(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    preferences::start(shell, window, cx);
    #[cfg(not(target_arch = "wasm32"))]
    if cx.has_global::<crate::desktop::Desktop>() {
        native::register(shell.words, cx);
        cli::start(shell, window, cx);
    }
}

pub(crate) fn cli_section(
    shell: &Shell,
    tokens: &vessel_app_kit::Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<vessel_app_kit::ui::Div> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        Some(cli::section(shell, tokens, cx))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _browser = (shell, tokens, cx);
        None
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn listen(cx: &mut gpui::App) {
    native::listen(cx);
}
