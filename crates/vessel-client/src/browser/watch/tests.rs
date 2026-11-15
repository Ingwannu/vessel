//! Scope/order fixtures only. No producer, PNG, paint or engine is fabricated.
use super::*;
use vessel_proto::browser::{
    BrowserHistory, BrowserIcon, BrowserLoad, BrowserTab, BrowserWatchEnd, PageText, ViewportSide,
    ViewportSize,
};
fn viewport(width: u32, height: u32) -> ViewportSize {
    ViewportSize {
        width: ViewportSide::try_from(width).unwrap(),
        height: ViewportSide::try_from(height).unwrap(),
    }
}
fn opening() -> (NodeId, BrowserWatched) {
    (
        NodeId::random(),
        BrowserWatched {
            subscription: SubscriptionId::new(7),
            revision: BrowserRevision::ZERO,
            tab: BrowserTab {
                browser_id: "00000000-0000-4000-8000-000000000001".parse().unwrap(),
                workspace: WorkspaceId::random(),
                url: "https://watch.invalid/".parse().unwrap(),
                title: PageText::try_from("Own descriptor fixture".to_owned()).unwrap(),
                load: BrowserLoad::Loading,
                history: BrowserHistory::Empty,
                viewport: BrowserViewport::Responsive(viewport(390, 844)),
                profile: BrowserProfile::General,
                icon: BrowserIcon::Globe,
            },
        },
    )
}
fn state(tab: BrowserTab, revision: BrowserRevision, sequence: BrowserUpdateSeq) -> BrowserUpdate {
    BrowserUpdate {
        sequence,
        change: BrowserChange::State { revision, tab },
    }
}
#[test]
fn scope_mismatch_poisoning_cannot_be_followed_by_a_valid_update() {
    let (node, opened) = opening();
    for mismatch in 0..4 {
        let mut cursor = Cursor::new(node, &opened);
        let mut tab = opened.tab.clone();
        match mismatch {
            0 => tab.workspace = WorkspaceId::random(),
            1 => tab.browser_id = "00000000-0000-4000-8000-000000000002".parse().unwrap(),
            2 => {
                tab.profile = BrowserProfile::Preview {
                    opening_session: vessel_proto::SessionId::random(),
                }
            }
            3 => {}
            _ => unreachable!(),
        }
        let source = if mismatch == 3 {
            NodeId::random()
        } else {
            node
        };
        let changed = state(tab, opened.revision, BrowserUpdateSeq::FIRST);
        assert_eq!(
            cursor.accept(source, opened.subscription, &changed),
            Err(WatchError::Scope)
        );
        let valid = state(opened.tab.clone(), opened.revision, BrowserUpdateSeq::FIRST);
        assert_eq!(
            cursor.accept(node, opened.subscription, &valid),
            Err(WatchError::Closed)
        );
    }
}
#[test]
fn changed_css_context_needs_a_new_revision_and_title_updates_do_not() {
    let (node, opened) = opening();
    let mut cursor = Cursor::new(node, &opened);
    let mut tab = opened.tab.clone();
    tab.title = PageText::try_from("Updated title".to_owned()).unwrap();
    let first = state(tab.clone(), opened.revision, BrowserUpdateSeq::FIRST);
    cursor.accept(node, opened.subscription, &first).unwrap();
    tab.viewport = BrowserViewport::Responsive(viewport(800, 600));
    let next = BrowserUpdateSeq::FIRST.next().unwrap();
    assert_eq!(
        cursor.accept(
            node,
            opened.subscription,
            &state(tab.clone(), opened.revision, next)
        ),
        Err(WatchError::Revision)
    );
    let mut cursor = Cursor::new(node, &opened);
    cursor
        .accept(
            node,
            opened.subscription,
            &state(
                tab.clone(),
                opened.revision.next().unwrap(),
                BrowserUpdateSeq::FIRST,
            ),
        )
        .unwrap();
    let end = BrowserUpdate {
        sequence: next,
        change: BrowserChange::Ended {
            end: BrowserWatchEnd::Revoked,
        },
    };
    cursor.accept(node, opened.subscription, &end).unwrap();
    assert_eq!(
        cursor.accept(node, opened.subscription, &end),
        Err(WatchError::Closed)
    );
}
#[test]
fn gaps_exhaustion_and_connection_close_are_not_a_fake_resume_cursor() {
    let (node, opened) = opening();
    let mut cursor = Cursor::new(node, &opened);
    let gap = state(
        opened.tab.clone(),
        opened.revision,
        BrowserUpdateSeq::FIRST.next().unwrap(),
    );
    assert_eq!(
        cursor.accept(node, opened.subscription, &gap),
        Err(WatchError::Sequence)
    );
    let mut cursor = Cursor::new(node, &opened);
    cursor.close();
    let first = state(opened.tab.clone(), opened.revision, BrowserUpdateSeq::FIRST);
    assert_eq!(
        cursor.accept(node, opened.subscription, &first),
        Err(WatchError::Closed)
    );
    let last: BrowserUpdateSeq = serde_json::from_str(&u64::MAX.to_string()).unwrap();
    let mut cursor = Cursor::new(node, &opened);
    cursor.phase = Phase::Open(last);
    assert_eq!(
        cursor.accept(
            node,
            opened.subscription,
            &state(opened.tab.clone(), opened.revision, last)
        ),
        Err(WatchError::Exhausted)
    );
    let mut cursor = Cursor::new(node, &opened);
    cursor.phase = Phase::Open(last);
    cursor
        .accept(
            node,
            opened.subscription,
            &BrowserUpdate {
                sequence: last,
                change: BrowserChange::Ended {
                    end: BrowserWatchEnd::Closed,
                },
            },
        )
        .unwrap();
}

fn frame(
    opened: &BrowserWatched,
    revision: BrowserRevision,
    sequence: BrowserUpdateSeq,
) -> BrowserUpdate {
    let image = vessel_proto::browser::ScreenshotPng::try_from(
        include_bytes!("fixtures/owned-page.png").to_vec(),
    )
    .unwrap();
    BrowserUpdate {
        sequence,
        change: BrowserChange::Frame {
            frame: vessel_proto::browser::BrowserFrame {
                browser_id: opened.tab.browser_id.clone(),
                workspace: opened.tab.workspace,
                revision,
                viewport: viewport(390, 844),
                image,
            },
        },
    }
}
#[test]
fn real_png_payload_still_needs_the_exact_announced_frame_context() {
    let (node, opened) = opening();
    let mut cursor = Cursor::new(node, &opened);
    let first = frame(&opened, opened.revision, BrowserUpdateSeq::FIRST);
    cursor.accept(node, opened.subscription, &first).unwrap();
    let mut wrong = frame(
        &opened,
        opened.revision,
        BrowserUpdateSeq::FIRST.next().unwrap(),
    );
    let BrowserChange::Frame { frame } = &mut wrong.change else {
        unreachable!()
    };
    frame.viewport = viewport(800, 600);
    assert_eq!(
        cursor.accept(node, opened.subscription, &wrong),
        Err(WatchError::Viewport)
    );
    let mut cursor = Cursor::new(node, &opened);
    cursor
        .accept(
            node,
            opened.subscription,
            &state(
                opened.tab.clone(),
                opened.revision.next().unwrap(),
                BrowserUpdateSeq::FIRST,
            ),
        )
        .unwrap();
    let old = frame(
        &opened,
        opened.revision,
        BrowserUpdateSeq::FIRST.next().unwrap(),
    );
    assert_eq!(
        cursor.accept(node, opened.subscription, &old),
        Err(WatchError::Revision)
    );
    let mut cursor = Cursor::new(node, &opened);
    assert_eq!(
        cursor.accept(node, SubscriptionId::new(8), &first),
        Err(WatchError::Scope)
    );
}
