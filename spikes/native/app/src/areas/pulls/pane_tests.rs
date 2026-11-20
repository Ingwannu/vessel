//! The pull request pane drawn from a read: each control is named for
//! assistive technology and folds say whether they are open (the app
//! reviewer's R1 on P3); a check's "Adding to chat" ends whatever the read
//! did while `pr.check` was out (R2).
use std::cell::RefCell;
use std::rc::Rc;

use chrono::{DateTime, Duration, TimeZone as _, Utc};
use gpui::{
    AppContext as _, Context, Entity, IntoElement, Modifiers, ParentElement as _, Render,
    Styled as _, TestAppContext, VisualTestContext, Window, size,
};
use vessel_app_kit::ui::px;
use vessel_app_kit::{Scheme, Tokens};
use vessel_app_view::workspace::problem::Problem;
use vessel_proto::forge::{
    CheckDetails, CheckRun, CheckState, Checks, ChecksState, ForgeAccess, ForgeKind,
    GetCheckDetails, MergeMethod, Mergeable, PullBody, PullNumber, PullRequest, PullRequestView,
    PullState, PullSummary, PullTimeline, PullTitle, RepoRef, ReviewDecision, ReviewState, Reviews,
    RunId, TimelineItem,
};
use vessel_proto::workspace::WorkspaceRoot;

use super::pane_attach::run_key;
use super::pane_timeline::TimelineRead;
use super::{Facts, FactsRead, Subject};
use crate::shell::{Shell, language_tests};

struct Probe {
    shell: Entity<Shell>,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let pane = self.shell.update(cx, |shell, cx| {
            let shell: &Shell = shell;
            super::pane::shown(
                shell,
                shell.areas.pulls.facts.as_ref(),
                &Tokens::of(Scheme::Dark),
                window,
                cx,
            )
        });
        gpui::div().size_full().flex().flex_col().child(pane)
    }
}

fn at(minute: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 19, 10, 0, 0)
        .unwrap()
        .checked_add_signed(Duration::minutes(minute))
        .unwrap()
}

fn run(name: &str, state: CheckState, id: u64) -> CheckRun {
    CheckRun {
        name: name.parse().unwrap(),
        state,
        url: Some(
            format!("https://github.com/acme/widgets/runs/{id}")
                .parse()
                .unwrap(),
        ),
        run: Some(RunId::new(id).unwrap()),
        workflow_run: None,
        workflow: Some("CI".parse().unwrap()),
        started_at: Some(at(0)),
        completed_at: Some(at(3)),
    }
}

/// Number 42 of `acme/widgets`: lint passed, test failed.
fn pull() -> PullRequest {
    let runs = vec![
        run("lint", CheckState::Passed, 7),
        run("test", CheckState::Failed, 8),
    ];
    PullRequest {
        summary: PullSummary {
            repo: RepoRef {
                kind: ForgeKind::Github,
                host: "github.com".parse().unwrap(),
                path: "acme/widgets".parse().unwrap(),
            },
            number: PullNumber::new(42).unwrap(),
            url: "https://github.com/acme/widgets/pull/42".parse().unwrap(),
            title: PullTitle::lossy("Add the widget"),
            state: PullState::Open,
            draft: false,
            head_commit: "0123456789abcdef0123456789abcdef01234567".parse().unwrap(),
            checks: ChecksState::Failing,
            review: ReviewDecision::None,
            mergeable: Mergeable::Mergeable,
            updated_at: at(10),
            tally: None,
        },
        author: None,
        head_branch: "feature".parse().unwrap(),
        base: "main".parse().unwrap(),
        checks: Checks {
            state: ChecksState::Failing,
            passed: 1,
            failed: 1,
            pending: 0,
            truncated: false,
            failing: runs.get(1).cloned().into_iter().collect(),
            runs,
        },
        reviews: Reviews {
            approved: false,
            changes_requested: false,
            commented: true,
            unresolved: 0,
        },
        created_at: at(0),
        merged_at: None,
        closed_at: None,
        merge_methods: vec![MergeMethod::Merge],
        read_at: at(10),
    }
}

fn timeline() -> PullTimeline {
    PullTimeline {
        access: ForgeAccess::Ready { login: None },
        number: Some(PullNumber::new(42).unwrap()),
        items: vec![TimelineItem::Comment {
            id: "IC_1".parse().unwrap(),
            author: Some("mkim".parse().unwrap()),
            body: PullBody::lossy("Should this cap at 30 seconds?"),
            created_at: at(5),
            url: Some(
                "https://github.com/acme/widgets/pull/42#issuecomment-1"
                    .parse()
                    .unwrap(),
            ),
            review: None,
            thread: None,
        }],
        truncated: false,
    }
}

fn facts() -> Facts {
    Facts {
        subject: Subject {
            machine: "fixture".to_owned(),
            session: "00000000-0000-4000-8000-000000000042".parse().unwrap(),
            root: WorkspaceRoot::Project {
                project: "00000000-0000-4000-8000-00000000000a".parse().unwrap(),
            },
        },
        read: FactsRead::Ready(Box::new(PullRequestView {
            access: ForgeAccess::Ready { login: None },
            pull_request: Some(Box::new(pull())),
            feedback: false,
        })),
        refreshing: false,
        seen: None,
        timeline: TimelineRead::Ready(Box::new(timeline())),
        adding: super::pane_attach::Adding::default(),
        choices: super::Choices::default(),
    }
}

fn setup(cx: &mut TestAppContext) -> (Entity<Shell>, &mut VisualTestContext) {
    language_tests::init(cx);
    let saved = Rc::new(RefCell::new(None));
    let window = cx.update(|cx| {
        cx.open_window(gpui::WindowOptions::default(), {
            let saved = Rc::clone(&saved);
            move |window, cx| {
                let shell = cx.new(|cx| {
                    let mut shell = Shell::without_io(window, cx);
                    // A draft's composer takes what the pane adds.
                    shell.route = crate::routes::Route::Draft;
                    shell.areas.pulls.facts = Some(facts());
                    shell
                });
                *saved.borrow_mut() = Some(shell.clone());
                let probe = cx.new(|_| Probe { shell });
                vessel_app_kit::root(probe, window, cx)
            }
        })
        .unwrap()
    });
    let cx = VisualTestContext::from_window(window.into(), cx).into_mut();
    cx.run_until_parked();
    cx.simulate_resize(size(px(440.0), px(1400.0)));
    cx.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    let shell = saved.borrow().as_ref().unwrap().clone();
    (shell, cx)
}

/// Every `aria` object of GPUI's real snapshot, as text.
fn arias(shell: &Entity<Shell>, cx: &mut VisualTestContext) -> Vec<String> {
    let handle = cx.update(|window, _| window.window_handle());
    assert!(cx.simulate_accessibility_activation(handle));
    cx.run_until_parked();
    shell.update(cx, |_, cx| cx.notify());
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let json = cx.update(|window, _| window.debug_a11y_tree_json().unwrap());
    json.split("\"aria\":")
        .skip(1)
        .filter_map(|rest| rest.split_once('}').map(|(aria, _)| aria.to_owned()))
        .collect()
}

/// The one control named `label`.
fn named<'a>(arias: &'a [String], label: &str) -> &'a str {
    let found: Vec<&String> = arias
        .iter()
        .filter(|aria| aria.contains(&format!(r#""label": "{label}""#)))
        .collect();
    assert_eq!(found.len(), 1, "one control named {label}");
    found.first().copied().unwrap()
}

#[gpui::test]
fn a_ready_panes_controls_are_named_and_its_folds_say_whether_open(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx);
    let said = arias(&shell, cx);
    let button = |label: &str| {
        let aria = named(&said, label);
        assert!(aria.contains(r#""Button""#), "{label} is a button");
        assert!(aria.contains(r#""Click""#), "{label} takes a click");
        aria.to_owned()
    };
    let link = |label: &str| {
        let aria = named(&said, label);
        assert!(aria.contains(r#""Link""#), "{label} is a link");
    };
    let _refresh = button("Refresh git and GitHub state");
    link("Open on GitHub");
    link("Add the widget #42");
    let head = button("Some checks were not successful. 1 failing, 1 successful checks");
    assert!(head.contains(r#""expanded": true"#));
    assert!(button("1 failing check").contains(r#""expanded": true"#));
    link("test");
    link("lint");
    assert!(button("Activity").contains(r#""expanded": true"#));
    assert!(button("mkim commented").contains(r#""expanded": true"#));
    let _kebab = button("Comment actions");
    let adds = said
        .iter()
        .filter(|aria| aria.contains(r#""label": "Add to chat""#))
        .count();
    // The failed check's and the comment's.
    assert_eq!(adds, 2);
    let _all = button("Add all to chat");

    // A press on the checks head folds the list, and it says so.
    let head = cx.debug_bounds("pr-pane-checks-head").unwrap();
    cx.simulate_click(head.center(), Modifiers::none());
    cx.run_until_parked();
    let said = arias(&shell, cx);
    let head = named(
        &said,
        "Some checks were not successful. 1 failing, 1 successful checks",
    );
    assert!(head.contains(r#""expanded": false"#));
}

#[gpui::test]
fn a_checks_adding_ends_when_the_read_failed_while_it_was_out(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx);
    let ask = GetCheckDetails {
        session: "00000000-0000-4000-8000-000000000042".parse().unwrap(),
        run: RunId::new(8).unwrap(),
    };
    shell.update(cx, |shell, _| {
        let facts = shell.areas.pulls.facts.as_mut().unwrap();
        let _new = facts.adding.checks.insert(run_key(8));
        // The node dropped: the whole read failed before the answer.
        facts.read = FactsRead::Failed(Problem::Unsent);
        shell.pulls_check_answered(
            "fixture",
            &ask,
            Ok(Box::new(CheckDetails {
                access: ForgeAccess::Ready { login: None },
                report: None,
            })),
        );
        let facts = shell.areas.pulls.facts.as_ref().unwrap();
        assert!(facts.adding.checks.is_empty(), "the spinner ends");
        assert!(facts.adding.pending.is_empty(), "nothing is added");
    });
}

#[gpui::test]
fn a_checks_adding_ends_when_its_run_left_the_list(cx: &mut TestAppContext) {
    let (shell, cx) = setup(cx);
    let ask = GetCheckDetails {
        session: "00000000-0000-4000-8000-000000000042".parse().unwrap(),
        run: RunId::new(8).unwrap(),
    };
    shell.update(cx, |shell, _| {
        let facts = shell.areas.pulls.facts.as_mut().unwrap();
        let _new = facts.adding.checks.insert(run_key(8));
        // The check ran again under a new run before the answer.
        let mut moved = pull();
        moved.checks.runs = vec![run("test", CheckState::Failed, 9)];
        facts.read = FactsRead::Ready(Box::new(PullRequestView {
            access: ForgeAccess::Ready { login: None },
            pull_request: Some(Box::new(moved)),
            feedback: false,
        }));
        shell.pulls_check_answered("fixture", &ask, Err(Problem::Unsent));
        let facts = shell.areas.pulls.facts.as_ref().unwrap();
        assert!(facts.adding.checks.is_empty(), "the spinner ends");
    });
    // The row's button takes a click again.
    let said = arias(&shell, cx);
    let adds: Vec<&String> = said
        .iter()
        .filter(|aria| aria.contains(r#""label": "Add to chat""#))
        .collect();
    assert!(adds.iter().all(|aria| aria.contains(r#""Click""#)));
}

#[gpui::test]
fn an_approval_without_words_is_a_link_with_its_own_menu(cx: &mut TestAppContext) {
    // An approval without words is a row that opens it: a link named by
    // its author and verb, with no menu of Copy.
    let (shell, cx) = setup(cx);
    shell.update(cx, |shell, _| {
        let facts = shell.areas.pulls.facts.as_mut().unwrap();
        if let TimelineRead::Ready(timeline) = &mut facts.timeline {
            timeline.items.push(TimelineItem::Review {
                id: "PRR_2".parse().unwrap(),
                author: Some("slee".parse().unwrap()),
                body: PullBody::lossy(""),
                created_at: at(6),
                url: Some(
                    "https://github.com/acme/widgets/pull/42#pullrequestreview-2"
                        .parse()
                        .unwrap(),
                ),
                state: ReviewState::Approved,
            });
        }
    });
    let said = arias(&shell, cx);
    assert!(named(&said, "slee approved").contains(r#""Link""#));
    let kebabs = said
        .iter()
        .filter(|aria| aria.contains(r#""label": "Comment actions""#))
        .count();
    assert_eq!(kebabs, 2, "the comment's and the approval's (Open)");
}
