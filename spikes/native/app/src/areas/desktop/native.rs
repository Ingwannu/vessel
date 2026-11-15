//! GPUI owns the macOS menu bar; other desktops draw the same commands via the kit.

use super::menus::{self, Item};
use gpui::{App, Menu, MenuItem, actions};
use vessel_app_view::localization::Translator;
use vessel_app_view::shortcuts::desktop::DesktopAction;

actions!(
    desktop,
    [
        NewWindow,
        CloseWindow,
        Quit,
        ZoomIn,
        ZoomOut,
        ActualSize,
        Undo,
        Redo,
        Cut,
        Copy,
        Paste,
        SelectAll,
        Fullscreen,
        Reload,
        ForceReload,
        Minimize,
        Maximize,
        Changelog,
        About,
        ReportIssue
    ]
);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Capture {
    Idle,
    Recording,
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct MenuIdentity {
    locale: vessel_app_view::localization::Locale,
    capture: Capture,
}
struct Registered(MenuIdentity);
impl gpui::Global for Registered {}

fn identity(shell: &crate::shell::Shell) -> MenuIdentity {
    MenuIdentity {
        locale: shell.words.locale(),
        capture: if !shell.desktop_commands_admitted() {
            Capture::Recording
        } else {
            Capture::Idle
        },
    }
}

pub(super) fn refresh(shell: &crate::shell::Shell, window: &gpui::Window, cx: &mut App) {
    if window.is_window_active()
        && cx
            .try_global::<Registered>()
            .is_none_or(|registered| registered.0 != identity(shell))
    {
        register(shell, cx);
    }
}

pub(super) fn register(shell: &crate::shell::Shell, cx: &mut App) {
    let words = shell.words;
    let w = words.desktop_shell();
    let recording = !shell.desktop_commands_admitted();
    let mut groups = vec![Menu::new("Vessel").items([
        MenuItem::action(w.about().to_owned(), About).disabled(recording),
        MenuItem::action(w.whats_new().to_owned(), Changelog).disabled(recording),
        MenuItem::separator(),
        MenuItem::action(w.quit().to_owned(), Quit).disabled(recording),
    ])];
    for group in [
        menus::Group::File,
        menus::Group::Edit,
        menus::Group::View,
        menus::Group::Window,
        menus::Group::Help,
    ] {
        let mut items = Vec::new();
        for row in menus::rows(shell, group) {
            if row.view.starts_group && !items.is_empty() {
                items.push(MenuItem::separator());
            }
            items.push(native_item(row));
        }
        groups.push(Menu::new(label(group, words).to_owned()).items(items));
    }
    cx.set_menus(groups);
    cx.set_global(Registered(identity(shell)));
}

fn label(group: menus::Group, words: &Translator) -> &str {
    let w = words.desktop_shell();
    match group {
        menus::Group::All => w.menu(),
        menus::Group::Context | menus::Group::Edit => w.edit(),
        menus::Group::File => w.file(),
        menus::Group::View => w.view(),
        menus::Group::Window => w.window(),
        menus::Group::Help => w.help(),
    }
}

fn native_item(row: menus::Row) -> MenuItem {
    // GPUI's public action registry is the same open boundary as input editing.
    MenuItem::Action {
        name: row.view.label,
        action: action(row.item),
        os_action: None,
        checked: false,
        disabled: row.view.disabled,
    }
}

fn action(item: Item) -> Box<dyn gpui::Action> {
    use vessel_app_view::desktop::HelpAction;
    match item {
        Item::Desktop(action) => match action {
            DesktopAction::NewWindow => Box::new(NewWindow),
            DesktopAction::CloseWindow => Box::new(CloseWindow),
            DesktopAction::ZoomIn => Box::new(ZoomIn),
            DesktopAction::ZoomOut => Box::new(ZoomOut),
            DesktopAction::ActualSize => Box::new(ActualSize),
            DesktopAction::Fullscreen => Box::new(Fullscreen),
        },
        // Native menu invocations must pass the same recorder admission before
        // dispatching kit editing actions to the current input. Passing the kit
        // action here would bypass that gate when native menu state is stale.
        Item::Edit(edit) => match edit {
            menus::Edit::Undo => Box::new(Undo),
            menus::Edit::Redo => Box::new(Redo),
            menus::Edit::Cut => Box::new(Cut),
            menus::Edit::Copy => Box::new(Copy),
            menus::Edit::Paste => Box::new(Paste),
            menus::Edit::SelectAll => Box::new(SelectAll),
        },
        Item::Reload => Box::new(Reload),
        Item::ForceReload => Box::new(ForceReload),
        Item::Minimize => Box::new(Minimize),
        Item::Maximize => Box::new(Maximize),
        Item::Quit => Box::new(Quit),
        Item::Help(help) => match help {
            HelpAction::Changelog => Box::new(Changelog),
            HelpAction::About => Box::new(About),
            HelpAction::ReportIssue => Box::new(ReportIssue),
        },
    }
}

pub(super) fn listen(cx: &mut App) {
    macro_rules! route {
        ($action:ident,$item:expr) => {
            let _registered = cx.on_action(|_: &$action, cx| invoke($item, cx));
        };
    }
    route!(NewWindow, Item::Desktop(DesktopAction::NewWindow));
    route!(Quit, Item::Quit);
    route!(CloseWindow, Item::Desktop(DesktopAction::CloseWindow));
    route!(ZoomIn, Item::Desktop(DesktopAction::ZoomIn));
    route!(ZoomOut, Item::Desktop(DesktopAction::ZoomOut));
    route!(ActualSize, Item::Desktop(DesktopAction::ActualSize));
    route!(Undo, Item::Edit(menus::Edit::Undo));
    route!(Redo, Item::Edit(menus::Edit::Redo));
    route!(Cut, Item::Edit(menus::Edit::Cut));
    route!(Copy, Item::Edit(menus::Edit::Copy));
    route!(Paste, Item::Edit(menus::Edit::Paste));
    route!(SelectAll, Item::Edit(menus::Edit::SelectAll));
    route!(Fullscreen, Item::Desktop(DesktopAction::Fullscreen));
    route!(Reload, Item::Reload);
    route!(ForceReload, Item::ForceReload);
    route!(Minimize, Item::Minimize);
    route!(Maximize, Item::Maximize);
    route!(
        Changelog,
        Item::Help(vessel_app_view::desktop::HelpAction::Changelog)
    );
    route!(
        About,
        Item::Help(vessel_app_view::desktop::HelpAction::About)
    );
    route!(
        ReportIssue,
        Item::Help(vessel_app_view::desktop::HelpAction::ReportIssue)
    );
}

fn invoke(item: Item, cx: &mut App) {
    // macOS keeps its application menu after the last window closes; there is
    // then no recorder or Shell to admit these two process-level commands.
    if cx.windows().is_empty() {
        if matches!(item, Item::Desktop(DesktopAction::NewWindow)) {
            crate::desktop::open_additional(cx);
            return;
        }
        if matches!(item, Item::Quit) {
            crate::desktop::request_quit(cx);
            return;
        }
    }
    crate::desktop::with_focused_shell(cx, move |shell, window, cx| {
        menus::invoke(shell, item, window, cx);
    });
}
