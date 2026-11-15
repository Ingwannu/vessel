//! Desktop menus and pending OS intents; authenticated destinations stay with owners.

#[cfg(not(target_arch = "wasm32"))]
mod actions;
#[cfg(not(target_arch = "wasm32"))]
mod menus;
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
}

impl DesktopArea {
    pub(crate) fn new(_cx: &mut gpui::App) -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            pending: None,
            #[cfg(not(target_arch = "wasm32"))]
            menu: vessel_app_kit::menu::MenuState::new(_cx),
            #[cfg(not(target_arch = "wasm32"))]
            group: menus::Group::File,
            #[cfg(not(target_arch = "wasm32"))]
            focus: None,
            #[cfg(not(target_arch = "wasm32"))]
            zoom: actions::Zoom::default(),
            #[cfg(not(target_arch = "wasm32"))]
            preferences:None,
        }
    }
}

pub(crate) fn pending(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
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
