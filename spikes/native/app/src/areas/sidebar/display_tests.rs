//! The Display menu's pages beside it on a wide window (TRACKER 791).
use super::*;
use gpui::TestAppContext;

#[gpui::test]
fn a_wide_display_menu_opens_a_page_beside_it_and_an_option_there_closes_it(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update_in(cx, |shell, window, cx| {
        let menu = &mut shell.areas.sidebar.display;
        menu.list.set_submenus(Some(&menu.pages));
        menu.list.open(None, window, cx);
        menu.page_list.open_beside();
        cx.notify();
    });
    cx.run_until_parked();
    // The Grouping row is drawn as its page's trigger.
    shell.read_with(cx, |shell, _| {
        assert!(
            shell
                .areas
                .sidebar
                .display
                .pages
                .trigger("grouping")
                .is_some()
        );
    });
    let before = shell.read_with(cx, |shell, _| shell.prefs.settings.sidebar.group);
    shell.update_in(cx, |shell, window, cx| {
        shell.areas.sidebar.display.pages.open("grouping", 0);
        // The option that is not in effect.
        let view = shell.prefs.settings.sidebar;
        let rows = DisplayPage::Grouping.rows(view, shell.t.lang.sidebar());
        let first_in_effect = rows
            .first()
            .is_some_and(|row| matches!(row.kind, RowKind::Choice { checked: true, .. }));
        let other = usize::from(first_in_effect);
        shell.display_choose_beside(DisplayPage::Grouping, other, window, cx);
    });
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        let menu = &shell.areas.sidebar.display;
        assert_ne!(shell.prefs.settings.sidebar.group, before);
        assert!(!menu.list.is_open());
        assert!(menu.pages.path().pages().is_empty());
    });
}

#[test]
fn each_page_has_a_name_and_the_name_leads_back_to_it() {
    for page in [DisplayPage::Grouping, DisplayPage::Sort] {
        let id = page_id(page).unwrap();
        assert_eq!(page_named(id), Some(page));
    }
    assert_eq!(page_id(DisplayPage::Root), None);
    assert_eq!(page_named("labels"), None);
}
