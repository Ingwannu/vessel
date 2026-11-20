//! A session's row to assistive technology (TRACKER 813), in GPUI's real
//! snapshot: the row is a button named by its title, its status slot a
//! status named by Paseo's bucket label, its kebab a button named for its
//! menu that a Click alone opens, once; its checks an image named by
//! their outcome, drawn as Paseo's ring (TRACKER 426).
use gpui::{TestAppContext, point};
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::ui::px;
use vessel_app_view::Attention;
use vessel_app_view::workspace::pull::CardChecks;
use vessel_proto::forge::{
    CheckTally, ChecksState, ForgeAccess, ForgeKind, Mergeable, PullNumber, PullState, PullSummary,
    PullTitle, RepoRef, ReviewDecision, SessionPullRequest,
};

use super::projects_tests::{named, setup, snapshot};
use super::row_meta::indicator;

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

/// An open pull request whose checks run, `tally` counted.
fn running(tally: CheckTally) -> SessionPullRequest {
    SessionPullRequest {
        access: ForgeAccess::Ready { login: None },
        summary: Some(PullSummary {
            repo: RepoRef {
                kind: ForgeKind::Github,
                host: "github.com".parse().unwrap(),
                path: "acme/shop".parse().unwrap(),
            },
            number: PullNumber::new(42).unwrap(),
            url: "https://github.com/acme/shop/pull/42".parse().unwrap(),
            title: PullTitle::lossy("Fix the cart total"),
            state: PullState::Open,
            draft: false,
            head_commit: "0123456789abcdef0123456789abcdef01234567".parse().unwrap(),
            checks: ChecksState::Pending,
            review: ReviewDecision::None,
            mergeable: Mergeable::Mergeable,
            updated_at: "2026-11-19T08:00:00Z".parse().unwrap(),
            tally: Some(tally),
        }),
        feedback: false,
    }
}

#[gpui::test]
fn a_rows_checks_are_named_by_their_outcome(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx, 2);
    shell.update(cx, |shell, cx| {
        let card = &mut shell.fleet.machines[0].sessions[0];
        card.detail.pull_request = Some(Box::new(running(CheckTally {
            passed: 3,
            failed: 0,
            pending: 1,
        })));
        cx.notify();
    });
    let arias = snapshot(&shell, cx);
    assert_eq!(named(&arias, "Image", "Checks running").len(), 1);
}

#[test]
fn the_ring_says_the_outcome_or_fills_by_quarters() {
    assert_eq!(indicator(CardChecks::Passed, 0), Glyph::ChecksPassed);
    assert_eq!(indicator(CardChecks::Failed, 2), Glyph::ChecksFailed);
    let pies = [0, 1, 2, 3, 4, 9].map(|quarters| indicator(CardChecks::Running, quarters));
    assert_eq!(
        pies,
        [
            Glyph::ChecksPie0,
            Glyph::ChecksPie1,
            Glyph::ChecksPie2,
            Glyph::ChecksPie3,
            Glyph::ChecksPie4,
            Glyph::ChecksPie4
        ]
    );
}
