//! A session's row to assistive technology (TRACKER 813), in GPUI's real
//! snapshot: the row is a button named by its title, its status slot a
//! status named by Paseo's bucket label, its kebab a button named for its
//! menu that a Click alone opens, once.
use gpui::{TestAppContext, point};
use vessel_app_kit::ui::px;
use vessel_app_view::Attention;

use super::projects_tests::{named, setup, snapshot};

const TITLES: [&str; 4] = [
    "Fix the cart total",
    "Sort the order list",
    "Summarise the meeting",
    "Write the release notes",
];

#[gpui::test]
fn a_row_is_a_button_named_by_its_title_with_its_status_and_menu(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx, 2);
    shell.update(cx, |shell, cx| {
        let sessions = &mut shell.fleet.machines[0].sessions;
        let attention = [
            Attention::Working,
            Attention::AtRest,
            Attention::NeedsYou,
            Attention::AtRest,
        ];
        for ((card, title), attention) in sessions.iter_mut().zip(TITLES).zip(attention) {
            title.clone_into(&mut card.title);
            card.attention = attention;
        }
        cx.notify();
    });
    let arias = snapshot(&shell, cx);
    for title in TITLES {
        assert_eq!(named(&arias, "Button", title).len(), 1, "{title}'s row");
    }
    // Each row's kebab: a button named for its menu, taking a Click.
    let kebabs = named(&arias, "Button", "Session actions");
    assert_eq!(kebabs.len(), TITLES.len());
    assert!(kebabs.iter().all(|aria| aria.contains(r#""Click""#)));
    // Each row's slot says its bucket (done included); the projects'
    // badges say theirs (shop working, notes needing input).
    let mut said: Vec<&str> = arias
        .iter()
        .filter(|aria| aria.contains(r#""role": "Status""#))
        .filter_map(|aria| {
            let (_, label) = aria.split_once(r#""label": ""#)?;
            label.split_once('"').map(|(label, _)| label)
        })
        .collect();
    said.sort_unstable();
    assert_eq!(
        said,
        [
            "Done",
            "Done",
            "Needs input",
            "Needs input",
            "Working",
            "Working"
        ]
    );
}

#[gpui::test]
fn a_click_alone_opens_a_rows_menu_once(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx, 2);
    shell.update_in(cx, |shell, window, cx| {
        let card = shell.fleet.machines[0].sessions[0].clone();
        let at = point(px(200.0), px(200.0));
        // No press went down first: assistive technology's Click.
        shell.row_menu_activate(&card, at, window, cx);
        assert!(shell.areas.sidebar.rows.is_for(&card));
        // The click that ends a pointer's press leaves it open.
        shell.row_menu_activate(&card, at, window, cx);
        assert!(shell.areas.sidebar.rows.is_for(&card));
    });
}
