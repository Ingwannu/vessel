//! Two real retained fields must never route text or Enter to their sibling.
use super::*;
use crate::shell::language_tests;
use gpui::{AppContext as _, Entity, IntoElement, Render, TestAppContext};
use vessel_app_kit::ui::{InteractiveElement as _, ParentElement as _, Styled as _, div};
use vessel_app_kit::{Scheme, Tokens};
use vessel_app_view::responses::{Problem as ResponseProblem, Request};
use vessel_proto::{AgentEvent, EventRecord, EventSeq, SessionEvent};

struct Probe {
    shell: Entity<Shell>,
    ids: [QuestionId; 2],
}
impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let ids = self.ids;
        self.shell.update(cx, |shell, cx| {
            let tokens = Tokens::of(Scheme::Dark);
            div().flex().flex_col().children(
                ids.into_iter()
                    .filter_map(|id| {
                        shell.areas.conversation.answering.get(id).map(|form| {
                            div()
                                .id(form.field.id().element_id("form-probe"))
                                .child(form.field.element(&tokens, false, cx))
                        })
                    })
                    .collect::<Vec<_>>(),
            )
        })
    }
}

#[gpui::test]
fn simultaneous_fields_keep_their_own_draft_and_enter_targets_the_second_request(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(make_probe);
    cx.update(|window, _| window.activate_window());
    for (index, text) in [(0, "first retained answer"), (1, "second retained answer")] {
        probe.update_in(cx, |probe, window, cx| {
            probe.shell.update(cx, |shell, cx| {
                shell
                    .areas
                    .conversation
                    .answering
                    .get(probe.ids[index])
                    .unwrap()
                    .field
                    .focus(window, cx);
            });
        });
        cx.run_until_parked();
        cx.simulate_input(text);
        cx.run_until_parked();
    }
    probe.update_in(cx, assert_drafts_retained);
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    probe.update_in(cx, |probe, _, cx| {
        probe.shell.read_with(cx, |shell, _| {
            assert_eq!(
                shell
                    .areas
                    .conversation
                    .response_failures
                    .get(Request::Question(probe.ids[0])),
                None
            );
            assert_eq!(
                shell
                    .areas
                    .conversation
                    .response_failures
                    .get(Request::Question(probe.ids[1])),
                Some(ResponseProblem::Offline)
            );
            assert_eq!(
                shell
                    .areas
                    .conversation
                    .answering
                    .get(probe.ids[0])
                    .unwrap()
                    .sending,
                None
            );
        })
    });
}

fn make_probe(window: &mut Window, cx: &mut Context<'_, Probe>) -> Probe {
    let mut ids = None;
    let shell = cx.new(|cx| {
        let mut shell = Shell::without_io(window, cx);
        shell
            .areas
            .conversation
            .open("scratch", vessel_proto::SessionId::random());
        super::language_tests::prepare(&mut shell, false, window, cx);
        let first = shell.areas.conversation.answering.as_ref().unwrap().id;
        let mut second = waiting_at::<SessionPlace>(&shell, first).unwrap();
        second.id = QuestionId::random();
        second.fields.truncate(1);
        let second_id = second.id;
        let _change = shell.areas.conversation.timeline.apply(&[EventRecord {
            seq: EventSeq::new(1),
            at: chrono::DateTime::UNIX_EPOCH,
            event: SessionEvent::Agent {
                event: AgentEvent::QuestionAsked(second),
            },
        }]);
        shell.question_prepare::<SessionPlace>(window, cx);
        assert_eq!(shell.areas.conversation.answering.len(), 2);
        ids = Some([first, second_id]);
        shell
    });
    Probe {
        shell,
        ids: ids.unwrap(),
    }
}

fn assert_drafts_retained(probe: &mut Probe, window: &mut Window, cx: &mut Context<'_, Probe>) {
    probe.shell.update(cx, |shell, cx| {
        let first = shell
            .areas
            .conversation
            .answering
            .get(probe.ids[0])
            .unwrap();
        let first_source = first.field.id();
        assert_eq!(first.step, 1);
        assert_eq!(
            first.drafts.get(&"question_1".parse().unwrap()),
            Some(&Draft::Text("first retained answer".into()))
        );
        let second = shell
            .areas
            .conversation
            .answering
            .get(probe.ids[1])
            .unwrap();
        let second_source = second.field.id();
        assert_ne!(first_source, second_source);
        assert_eq!(second.step, 0);
        assert_eq!(
            second.drafts.get(&"question_0".parse().unwrap()),
            Some(&Draft::Text("second retained answer".into()))
        );
        shell.question_prepare::<SessionPlace>(window, cx);
        assert_eq!(
            shell
                .areas
                .conversation
                .answering
                .get(probe.ids[0])
                .unwrap()
                .field
                .id(),
            first_source
        );
        assert_eq!(
            shell
                .areas
                .conversation
                .answering
                .get(probe.ids[1])
                .unwrap()
                .field
                .id(),
            second_source
        );
    });
}
