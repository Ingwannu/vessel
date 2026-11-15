//! The real shell receives metadata and owns pending navigation; no node is started.

use gpui::{AppContext as _, Entity, Render, TestAppContext, VisualTestContext};
use vessel_app_kit::ui::{
    Context, IntoElement as _, ParentElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{Scheme, Tokens};
use vessel_app_view::prefs::{ChatOutline, PreferenceChange};
use vessel_proto::SessionId;
use vessel_proto::prompt_index::{PromptIndexPage, PromptLocation, PromptPreview};

use super::*;
use crate::shell::language_tests;

struct Probe {
    shell: Entity<Shell>,
}

impl Render for Probe {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        self.shell.update(cx, |shell, cx| {
            let list = shell
                .areas
                .conversation
                .list
                .element(|_, _, _| div().into_any_element());
            div()
                .relative()
                .w(px(1_100.0))
                .h(px(600.0))
                .child(list)
                .children(super::super::outline::rail(
                    shell,
                    &Tokens::of(Scheme::Dark),
                    window,
                    cx,
                ))
                .into_any_element()
        })
    }
}

fn harness(cx: &mut TestAppContext) -> (Entity<Probe>, &mut VisualTestContext) {
    language_tests::init(cx);
    cx.add_window_view(|window, cx| {
        let shell = cx.new(|cx| {
            let mut shell = Shell::without_io(window, cx);
            let session = SessionId::random();
            shell.areas.conversation.open("machine", session);
            shell.areas.session.open = Some(crate::areas::session::Open {
                machine: "machine".to_owned(),
                session,
                title: "Test".to_owned(),
                agent: "Test".to_owned(),
                structured: true,
                live: false,
            });
            shell.prompt_index_refresh();
            shell
        });
        cx.observe(&shell, |_, _, cx| cx.notify()).detach();
        Probe { shell }
    })
}

fn update<R>(
    probe: &Entity<Probe>,
    cx: &mut VisualTestContext,
    f: impl FnOnce(&mut Shell, &mut Window, &mut Context<'_, Shell>) -> R,
) -> R {
    probe.update_in(cx, |probe, window, cx| {
        probe.shell.update(cx, |shell, cx| f(shell, window, cx))
    })
}

fn seed(shell: &mut Shell, cx: &mut Context<'_, Shell>) -> (IndexTarget, PromptIndexEpoch) {
    let index = &mut shell.areas.conversation.prompts.index;
    let target = index.target().unwrap().clone();
    let ask = index.restart().unwrap();
    let epoch = PromptIndexEpoch::new(EventSeq::ZERO, None).unwrap();
    let page = PromptIndexPage::new(
        target.session,
        epoch,
        [10, 20]
            .map(|seq| PromptLocation {
                seq: EventSeq::new(seq),
                timestamp: chrono::DateTime::UNIX_EPOCH,
                preview: PromptPreview::from_text("Prompt before its body"),
            })
            .into(),
        None,
    )
    .unwrap();
    shell.prompt_index_answered(
        IndexAnswer {
            ask,
            result: Ok(page),
        },
        cx,
    );
    (target, epoch)
}

#[gpui::test]
fn prompt_navigation_rejects_stale_painted_identity_and_bounds_an_unloaded_jump(
    cx: &mut TestAppContext,
) {
    let (probe, cx) = harness(cx);
    update(&probe, cx, |shell, _, cx| {
        let (target, epoch) = seed(shell, cx);
        let mut replaced = target.clone();
        replaced.node = Some(vessel_proto::NodeId::random());
        shell.prompt_selected(&replaced, EventSeq::new(10), Some(epoch), cx);
        assert_eq!(shell.areas.conversation.reveal, None);
        let old_epoch = PromptIndexEpoch::new(EventSeq::new(1), None).unwrap();
        shell.prompt_selected(&target, EventSeq::new(10), Some(old_epoch), cx);
        assert_eq!(shell.areas.conversation.reveal, None);
        shell.prompt_selected(&target, EventSeq::new(30), Some(epoch), cx);
        assert_eq!(shell.areas.conversation.reveal, None);
        shell.prompt_selected(&target, EventSeq::new(10), Some(epoch), cx);
        assert_eq!(shell.areas.conversation.reveal, Some(EventSeq::new(10)));
        assert!(shell.areas.conversation.navigation_held());
    });
    cx.run_until_parked();
    cx.executor().advance_clock(JUMP_DEADLINE);
    cx.run_until_parked();
    update(&probe, cx, |shell, _, _| {
        assert_eq!(shell.areas.conversation.reveal, None);
        assert!(shell.areas.conversation.prompts.jump.is_none());
        assert_eq!(
            shell.notice.as_deref(),
            Some(shell.words.chat_find().reveal())
        );
    });
}

#[gpui::test]
fn prompt_navigation_deadline_cannot_cancel_a_replacement_destination(cx: &mut TestAppContext) {
    let (probe, cx) = harness(cx);
    update(&probe, cx, |shell, _, cx| {
        let _metadata = seed(shell, cx);
        shell.prompt_jump(EventSeq::new(10), cx);
    });
    cx.run_until_parked();
    cx.executor().advance_clock(JUMP_DEADLINE / 2);
    update(&probe, cx, |shell, _, cx| {
        shell.prompt_jump(EventSeq::new(20), cx)
    });
    cx.run_until_parked();
    cx.executor().advance_clock(JUMP_DEADLINE / 2);
    cx.run_until_parked();
    update(&probe, cx, |shell, _, _| {
        assert_eq!(shell.areas.conversation.reveal, Some(EventSeq::new(20)));
        assert!(shell.notice.is_none());
    });
}

#[gpui::test]
fn prompt_navigation_outline_preference_removes_and_restores_the_actual_rail(
    cx: &mut TestAppContext,
) {
    let (probe, cx) = harness(cx);
    update(&probe, cx, |shell, _, cx| {
        let _metadata = seed(shell, cx);
    });
    cx.run_until_parked();
    update(&probe, cx, |_, _, cx| cx.notify());
    cx.run_until_parked();
    assert!(cx.debug_bounds("outline-rail").is_some());
    update(&probe, cx, |shell, window, cx| {
        shell.prefer(&PreferenceChange::ChatOutline(ChatOutline::Off), window, cx);
        assert!(shell.areas.conversation.prompt_target().is_none());
    });
    cx.run_until_parked();
    assert!(cx.debug_bounds("outline-rail").is_none());
    update(&probe, cx, |shell, window, cx| {
        shell.prefer(&PreferenceChange::ChatOutline(ChatOutline::On), window, cx);
        let _metadata = seed(shell, cx);
    });
    cx.run_until_parked();
    assert!(cx.debug_bounds("outline-rail").is_some());
}

#[gpui::test]
fn prompt_navigation_refreshes_metadata_when_another_chunk_extends_the_same_prompt(
    cx: &mut TestAppContext,
) {
    use vessel_app_view::prompt_index::IndexPhase;
    use vessel_proto::ContentBlock;

    let (probe, cx) = harness(cx);
    update(&probe, cx, |shell, _, cx| {
        shell.route = crate::routes::Route::Session;
        let session = shell.areas.conversation.shown.as_ref().unwrap().session;
        for seq in [1, 2] {
            let _metadata = seed(shell, cx);
            let records = [EventRecord {
                seq: EventSeq::new(seq),
                at: chrono::DateTime::UNIX_EPOCH,
                event: SessionEvent::Agent {
                    event: AgentEvent::UserMessageChunk {
                        content: ContentBlock::Text {
                            text: "continued".to_owned(),
                        },
                    },
                },
            }];
            let _changed = shell.areas.conversation.take(&session, &records);
            shell.prompt_index_records(&records);
            assert!(matches!(
                shell.areas.conversation.prompts.index.phase(),
                IndexPhase::Reading(_)
            ));
        }
        assert_eq!(shell.areas.conversation.outline.len(), 1);
    });
}
