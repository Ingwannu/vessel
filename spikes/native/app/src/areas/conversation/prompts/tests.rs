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
            delivery: None,
            result: Ok(page),
        },
        cx,
    );
    (target, epoch)
}

#[gpui::test]
fn metadata_success_without_an_actor_guard_cannot_install_addresses(cx: &mut TestAppContext) {
    use vessel_app_view::prompt_index::{IndexPhase, IndexProblem};
    let (probe, cx) = harness(cx);
    update(&probe, cx, |shell, _, cx| {
        let ask = shell.areas.conversation.prompts.index.restart().unwrap();
        let page = PromptIndexPage::new(
            ask.target.session,
            PromptIndexEpoch::new(EventSeq::ZERO, None).unwrap(),
            vec![PromptLocation {
                seq: EventSeq::new(10),
                timestamp: chrono::DateTime::UNIX_EPOCH,
                preview: PromptPreview::from_text("not an admitted server reply"),
            }],
            None,
        )
        .unwrap();
        // This is a rejection fixture, not a fake valid lease or wire success.
        shell.prompt_index_answered(
            IndexAnswer {
                ask,
                delivery: None,
                result: Ok(page),
            },
            cx,
        );
        assert_eq!(
            shell.areas.conversation.prompts.index.phase(),
            IndexPhase::Failed(IndexProblem::Connection)
        );
        assert!(shell.areas.conversation.prompts.index.entries().is_none());
    });
}

#[gpui::test]
fn prompt_navigation_rejects_stale_painted_identity_and_bounds_an_unloaded_jump(
    cx: &mut TestAppContext,
) {
    let (probe, cx) = harness(cx);
    update(&probe, cx, |shell, _, cx| {
        let (target, epoch) = seed(shell, cx);
        let mut replaced = target.clone();
        replaced.connection = Some(
            vessel_app_view::connection::ConnectionEpochs::default()
                .admit(target.machine.clone(), vessel_proto::NodeId::random())
                .unwrap(),
        );
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
        shell.prompt_jump(EventSeq::new(10), cx);
        shell.prefer(&PreferenceChange::ChatOutline(ChatOutline::Off), window, cx);
        assert!(shell.areas.conversation.prompt_target().is_none());
        assert_eq!(shell.areas.conversation.reveal, None);
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

#[gpui::test]
fn prompt_navigation_retries_after_the_same_node_is_readmitted(cx: &mut TestAppContext) {
    use vessel_app_view::connection::{ConnectionEpochs, MachineKey};
    use vessel_app_view::prompt_index::{IndexPhase, IndexProblem};
    use vessel_app_view::{Machine, Reach};
    use vessel_proto::{NodeId, PROTOCOL_VERSION};

    let (probe, cx) = harness(cx);
    update(&probe, cx, |shell, _, cx| {
        shell.route = crate::routes::Route::Session;
        let node = NodeId::random();
        let mut admissions = ConnectionEpochs::default();
        let original = admissions
            .admit(MachineKey::from_book("machine"), node)
            .unwrap();
        shell.fleet.machines = vec![Machine {
            name: "machine".to_owned(),
            reach: Reach::Online,
            sessions: Vec::new(),
            agents: Vec::new(),
            node: Some(node),
            connection_epoch: Some(original.epoch()),
            protocol: Some(PROTOCOL_VERSION),
            version: None,
        }];
        shell.prompt_index_refresh();
        let ask = shell.areas.conversation.prompts.index.restart().unwrap();
        shell.prompt_index_answered(
            IndexAnswer {
                ask,
                delivery: None,
                result: Err(IndexProblem::Unsupported),
            },
            cx,
        );
        assert_eq!(
            shell.areas.conversation.prompts.index.phase(),
            IndexPhase::Failed(IndexProblem::Unsupported)
        );
        let reconnected = admissions
            .admit(MachineKey::from_book("machine"), node)
            .unwrap();
        shell.fleet.machines[0].connection_epoch = Some(reconnected.epoch());
        shell.prompt_index_fleet_changed();
        assert!(matches!(
            shell.areas.conversation.prompts.index.phase(),
            IndexPhase::Reading(_)
        ));
    });
}

#[gpui::test]
fn origin_navigation_holds_and_retires_with_the_same_resource_tree(cx: &mut TestAppContext) {
    use super::super::origin::Origin;
    use vessel_app_view::prompt_index::{IndexPhase, IndexProblem};
    let (probe, cx) = harness(cx);
    update(&probe, cx, |shell, _, cx| {
        shell.areas.conversation.clear_prompts();
        assert!(!shell.areas.conversation.navigation_held());
        let mut inner = ConversationArea::new(cx);
        let ask = inner
            .prompts
            .index
            .refresh(
                IndexTarget {
                    machine: MachineKey::from_book("machine"),
                    connection: None,
                    session: SessionId::random(),
                },
                None,
            )
            .unwrap();
        let mut origin = ConversationArea::new(cx);
        origin.origin = Origin::Shown(Box::new(inner));
        shell.areas.conversation.origin = Origin::Shown(Box::new(origin));
        assert!(
            shell.areas.conversation.navigation_held(),
            "nested origin read must Hold"
        );
        shell.areas.conversation.navigation_retired();
        assert!(!shell.areas.conversation.navigation_held());
        let inner = shell
            .areas
            .conversation
            .origin
            .area_mut()
            .unwrap()
            .origin
            .area_mut()
            .unwrap();
        assert_eq!(
            inner.prompts.index.phase(),
            IndexPhase::Failed(IndexProblem::Connection)
        );
        let next = inner.prompts.index.restart().unwrap();
        assert_ne!(
            next.ticket, ask.ticket,
            "retirement must preserve the original ticket source"
        );
    });
}
