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
        Fullscreen,
        Reload,
        ForceReload,
        Minimize,
        Maximize,
        Changelog
    ]
);

pub(super) fn register(words: &Translator, cx: &mut App) {
    let w = words.desktop_shell();
    cx.set_menus([
        Menu::new("Vessel").items([
            MenuItem::action(w.whats_new().to_owned(), Changelog),
            MenuItem::separator(),
            MenuItem::action(w.quit().to_owned(), Quit),
        ]),
        Menu::new(w.file().to_owned()).items([
            MenuItem::action(w.new_window().to_owned(), NewWindow),
            MenuItem::action(w.close_window().to_owned(), CloseWindow),
        ]),
        Menu::new(w.edit().to_owned()).items(edit_items(words)),
        Menu::new(w.view().to_owned()).items([
            MenuItem::action(w.zoom_in().to_owned(), ZoomIn),
            MenuItem::action(w.zoom_out().to_owned(), ZoomOut),
            MenuItem::action(w.actual_size().to_owned(), ActualSize),
            MenuItem::separator(),
            MenuItem::action(w.reload().to_owned(), Reload),
            MenuItem::action(w.force_reload().to_owned(), ForceReload),
            MenuItem::separator(),
            MenuItem::action(w.fullscreen().to_owned(), Fullscreen),
        ]),
        Menu::new(w.window().to_owned()).items([
            MenuItem::action(w.minimize().to_owned(), Minimize),
            MenuItem::action(w.maximize().to_owned(), Maximize),
        ]),
    ]);
}

pub(super) fn listen(cx: &mut App) {
    let _new_window = cx.on_action(|_: &NewWindow, cx| crate::desktop::open_additional(cx));
    let _quit = cx.on_action(|_: &Quit, cx| crate::desktop::request_quit(cx));
    macro_rules! route {
        ($action:ident,$item:expr) => {
            let _registered = cx.on_action(|_: &$action, cx| invoke($item, cx));
        };
    }
    route!(CloseWindow, Item::Desktop(DesktopAction::CloseWindow));
    route!(ZoomIn, Item::Desktop(DesktopAction::ZoomIn));
    route!(ZoomOut, Item::Desktop(DesktopAction::ZoomOut));
    route!(ActualSize, Item::Desktop(DesktopAction::ActualSize));
    route!(Fullscreen, Item::Desktop(DesktopAction::Fullscreen));
    route!(Reload, Item::Reload);
    route!(ForceReload, Item::ForceReload);
    route!(Minimize, Item::Minimize);
    route!(Maximize, Item::Maximize);
    route!(Changelog, Item::Changelog);
}

fn invoke(item: Item, cx: &mut App) {
    crate::desktop::with_focused_shell(cx, move |shell, window, cx| {
        menus::invoke(shell, item, None, window, cx);
    });
}

fn edit_items(words: &Translator) -> Vec<MenuItem> {
    use vessel_app_kit::editing;
    let w = words.desktop_shell();
    vec![
        MenuItem::action(w.undo().to_owned(), editing::Undo),
        MenuItem::action(w.redo().to_owned(), editing::Redo),
        MenuItem::separator(),
        MenuItem::action(w.cut().to_owned(), editing::Cut),
        MenuItem::action(w.copy().to_owned(), editing::Copy),
        MenuItem::action(w.paste().to_owned(), editing::Paste),
        MenuItem::separator(),
        MenuItem::action(w.select_all().to_owned(), editing::SelectAll),
    ]
}
