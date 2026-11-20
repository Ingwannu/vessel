//! A session row's hover card (TRACKER 810) in the real sidebar: the
//! pointer on a row shows its details as a group named "Session details",
//! with Paseo's copy rows and links; a press on a copy row puts the fact
//! on the clipboard; no card while the row's menu is open; none carried
//! back with a sidebar hidden and shown again (the app reviewer's R1).
use gpui::{Modifiers, TestAppContext, VisualTestContext, point, size};
use vessel_app_kit::ui::px;
use vessel_app_link::Link;
use vessel_app_view::{Card, Fleet, Lang, Machine, Reach};
use vessel_proto::{HarnessKind, SessionId, SessionInfo, SessionMode, SessionSpec};
use vessel_proto::forge::{
    ChecksState, ForgeAccess, ForgeKind, Mergeable, PullNumber, PullState, PullSummary, PullTitle,
    RepoRef, ReviewDecision, SessionPullRequest,
};

use super::key;
use crate::areas::sidebar::projects_tests::{named, setup, snapshot};

fn pull() -> SessionPullRequest {
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
            checks: ChecksState::Failing,
            review: ReviewDecision::None,
            mergeable: Mergeable::Mergeable,
            updated_at: "2026-11-19T08:00:00Z".parse().unwrap(),
        }),
        feedback: false,
    }
}

/// The first session: titled, on a branch, in a folder, with a pull
/// request whose checks fail.
fn dressed(cx: &mut VisualTestContext, shell: &gpui::Entity<crate::shell::Shell>) -> Card {
    shell.update(cx, |shell, cx| {
        let card = &mut shell.fleet.machines[0].sessions[0];
        "Fix the cart total".clone_into(&mut card.title);
        card.detail.branch = Some("fix-cart".to_owned());
        card.detail.cwd = Some("/home/dev/work/shop".to_owned());
        card.detail.pull_request = Some(Box::new(pull()));
        cx.notify();
        card.clone()
    })
}

/// The pointer onto `card`'s row, where the sidebar last drew it.
fn hover_row(cx: &mut VisualTestContext, shell: &gpui::Entity<crate::shell::Shell>, card: &Card) {
    cx.run_until_parked();
    let bounds = shell
        .read_with(cx, |shell, _| {
            shell
                .areas
                .sidebar
                .place(&format!("card:{}", key(card)))
                .get()
        })
        .unwrap();
    cx.simulate_mouse_move(bounds.center(), None, Modifiers::none());
    cx.run_until_parked();
}

#[gpui::test]
fn a_rows_card_says_its_details_with_paseos_rows(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx, 2);
    let card = dressed(cx, &shell);
    hover_row(cx, &shell, &card);
    shell.read_with(cx, |shell, _| {
        assert_eq!(
            shell.areas.sidebar.cards.shown().as_deref(),
            Some(key(&card).as_str())
        );
    });
    let arias = snapshot(&shell, cx);
    assert_eq!(named(&arias, "Group", "Session details").len(), 1);
    assert_eq!(named(&arias, "Link", "Pull request #42").len(), 1);
    assert_eq!(named(&arias, "Button", "Copy branch name").len(), 1);
    assert_eq!(named(&arias, "Button", "Copy path").len(), 1);
    assert_eq!(named(&arias, "Link", "Checks: failed").len(), 1);
}

#[gpui::test]
fn a_copy_row_puts_its_fact_on_the_clipboard(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx, 2);
    let card = dressed(cx, &shell);
    hover_row(cx, &shell, &card);
    let branch = cx.debug_bounds("hover-card-branch").unwrap();
    cx.simulate_mouse_move(branch.center(), None, Modifiers::none());
    cx.simulate_click(branch.center(), Modifiers::none());
    cx.run_until_parked();
    let copied = cx.read_from_clipboard().and_then(|item| item.text());
    assert_eq!(copied.as_deref(), Some("fix-cart"));
    // The folder copies its path as the node wrote it.
    let folder = cx.debug_bounds("hover-card-folder").unwrap();
    cx.simulate_click(folder.center(), Modifiers::none());
    cx.run_until_parked();
    let copied = cx.read_from_clipboard().and_then(|item| item.text());
    assert_eq!(copied.as_deref(), Some("/home/dev/work/shop"));
}

#[gpui::test]
fn no_card_while_the_rows_menu_is_open(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx, 2);
    let card = dressed(cx, &shell);
    hover_row(cx, &shell, &card);
    shell.update_in(cx, |shell, window, cx| {
        shell.row_menu_open(&card, point(px(100.0), px(100.0)), window, cx);
        assert!(shell.areas.sidebar.cards.shown().is_none());
    });
    // Hovered again with the menu open: still none.
    cx.simulate_mouse_move(point(px(900.0), px(700.0)), None, Modifiers::none());
    hover_row(cx, &shell, &card);
    shell.read_with(cx, |shell, _| {
        assert!(shell.areas.sidebar.cards.shown().is_none());
    });
}

#[gpui::test]
fn no_card_comes_back_with_the_sidebar_when_the_pointer_moved_off(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (link, _sent) = Link::detached(8).unwrap();
    let (shell, cx) =
        cx.add_window_view(move |window, cx| crate::shell::Shell::with_test_link(window, cx, link));
    cx.simulate_resize(size(px(1280.0), px(800.0)));
    let spec = SessionSpec::new(
        HarnessKind::ClaudeCode,
        SessionMode::Structured {
            model: None,
            resume: None,
            options: Vec::new(),
        },
    );
    let info = SessionInfo::new(SessionId::random(), spec, "2026-11-19T08:00:00Z".parse().unwrap());
    let card = Card::from_session("fixture", &info);
    shell.update(cx, |shell, cx| {
        shell.t = Lang::En.text();
        shell.fleet = Fleet {
            machines: vec![Machine {
                connection_epoch: None,
                node: None,
                name: "fixture".to_owned(),
                reach: Reach::Online,
                sessions: vec![card.clone()],
                agents: Vec::new(),
                protocol: Some(vessel_proto::PROTOCOL_VERSION),
                version: None,
            }],
            read_at: None,
        };
        cx.notify();
    });
    hover_row(cx, &shell, &card);
    shell.read_with(cx, |shell, _| assert!(shell.areas.sidebar.cards.shown().is_some()));
    // Hidden by its key, the pointer into the conversation, shown again.
    shell.update_in(cx, |shell, window, cx| shell.toggle_sidebar(window, cx));
    cx.run_until_parked();
    cx.simulate_mouse_move(point(px(900.0), px(500.0)), None, Modifiers::none());
    cx.run_until_parked();
    shell.update_in(cx, |shell, window, cx| shell.toggle_sidebar(window, cx));
    cx.run_until_parked();
    shell.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| assert!(shell.areas.sidebar.cards.shown().is_none()));
    assert!(cx.debug_bounds("hover-card").is_none());
}
