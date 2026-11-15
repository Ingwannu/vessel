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
    pub(crate) pending: vessel_app_link::desktop::instance::Inbox,
    #[cfg(not(target_arch = "wasm32"))]
    menu: vessel_app_kit::menu::MenuState,
    #[cfg(not(target_arch = "wasm32"))]
    group: menus::Group,
    #[cfg(not(target_arch = "wasm32"))]
    edit: vessel_app_view::desktop::EditContext,
    #[cfg(not(target_arch = "wasm32"))]
    menu_cycle: std::rc::Rc<()>,
    #[cfg(not(target_arch = "wasm32"))]
    edit_recipient: Option<gpui::WeakFocusHandle>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) zoom: vessel_app_view::desktop::zoom::ZoomLevel,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) preferences: Option<vessel_app_link::desktop::preferences::Client>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) receipts: std::cell::RefCell<crate::desktop::preferences::ReceiptCapture>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) rollback: Option<vessel_app_link::Prefs>,
    #[cfg(not(target_arch = "wasm32"))]
    cli: cli::CliPanel,
}

impl DesktopArea {
    pub(crate) fn new(cx: &mut gpui::App) -> Self {
        #[cfg(target_arch = "wasm32")]
        let _browser = cx;
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            pending: vessel_app_link::desktop::instance::Inbox::default(),
            #[cfg(not(target_arch = "wasm32"))]
            menu: vessel_app_kit::menu::MenuState::new(cx),
            #[cfg(not(target_arch = "wasm32"))]
            group: menus::Group::File,
            #[cfg(not(target_arch = "wasm32"))]
            edit: vessel_app_view::desktop::EditContext::default(),
            #[cfg(not(target_arch = "wasm32"))]
            menu_cycle: std::rc::Rc::new(()),
            #[cfg(not(target_arch = "wasm32"))]
            edit_recipient: None,
            #[cfg(not(target_arch = "wasm32"))]
            zoom: vessel_app_view::desktop::zoom::ZoomLevel::default(),
            #[cfg(not(target_arch = "wasm32"))]
            preferences: None,
            #[cfg(not(target_arch = "wasm32"))]
            receipts: std::cell::RefCell::default(),
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
    {
        native::refresh(shell, window, cx);
        crate::desktop::drain_pending(shell, window, cx);
    }
    #[cfg(target_arch = "wasm32")]
    let _browser = (shell, window, cx);
}

/// The selection owner admits the popup; clipboard semantics stay with its focus.
pub(crate) fn open_edit_context(
    shell: &mut Shell,
    position: gpui::Point<gpui::Pixels>,
    edit: vessel_app_view::desktop::EditContext,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if !shell.desktop_commands_admitted() {
            return false;
        }
        menus::open_context(shell, position, edit, window, cx);
        true
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _browser = (shell, position, edit, window, cx);
        false
    }
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
        native::register(shell, cx);
        cli::start(shell, window, cx);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn cli_section(
    shell: &Shell,
    tokens: &vessel_app_kit::Tokens,
    cx: &mut Context<'_, Shell>,
) -> vessel_app_kit::ui::Div {
    cli::section(shell, tokens, cx)
}

impl Shell {
    /// Help menu and sidebar use the same destinations and external opener.
    pub(crate) fn run_desktop_help(
        &mut self,
        action: vessel_app_view::desktop::HelpAction,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        use vessel_app_view::desktop::HelpAction;
        match action {
            HelpAction::Changelog => self.open_changelog(window, cx),
            HelpAction::About => {
                self.open_settings(Some(vessel_app_view::settings::Section::About), window, cx);
            }
            HelpAction::ReportIssue => {
                cx.open_url(concat!(env!("CARGO_PKG_REPOSITORY"), "/issues/new/choose"));
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn listen(cx: &mut gpui::App) {
    native::listen(cx);
}
