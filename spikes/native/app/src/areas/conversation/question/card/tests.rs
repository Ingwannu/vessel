//! A description is readable without selecting its parent option.
use super::*;
use crate::areas::conversation::{SessionPlace, question::language_tests::prepare};
use crate::shell::language_tests;
use gpui::{AppContext as _, Entity, IntoElement, Modifiers, Render, TestAppContext, point};
use vessel_app_view::timeline::Draft;

struct Probe {
    shell: Entity<Shell>,
}
impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.shell.update(cx, |shell, cx| {
            let form = shell.areas.conversation.answering.as_ref().unwrap();
            option_row::<SessionPlace>(
                (0, form.cursor(), form.field.id()),
                Choice {
                    value: "a".into(),
                    title: "**A**".into(),
                    description: Some("Read this **description** before choosing.".into()),
                },
                false,
                false,
                false,
                &Tokens::of(vessel_app_kit::Scheme::Dark),
                cx,
            )
            .w(px(360.0))
            .debug_selector(|| "question-option-row".to_owned())
        })
    }
}

#[gpui::test]
fn reading_a_description_does_not_select_but_clicking_its_markdown_label_does(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| Probe {
        shell: cx.new(|cx| {
            let mut shell = Shell::without_io(window, cx);
            prepare(&mut shell, true, window, cx);
            shell
        }),
    });
    cx.run_until_parked();
    let description = cx.debug_bounds("question-option-description").unwrap();
    cx.simulate_click(description.center(), Modifiers::default());
    probe.update_in(cx, |probe, _, cx| {
        probe.shell.read_with(cx, |shell, _| {
            let form = shell.areas.conversation.answering.as_ref().unwrap();
            assert_eq!(form.step, 1);
            assert_eq!(
                form.drafts.get(&"question_1".parse().unwrap()),
                Some(&Draft::Picked(Vec::new()))
            );
        });
    });
    let label = cx.debug_bounds("question-option-title").unwrap();
    cx.simulate_click(
        point(
            px(label.left().as_f32().mul_add(1.0, 4.0)),
            label.center().y,
        ),
        Modifiers::default(),
    );
    probe.update_in(cx, |probe, _, cx| {
        probe.shell.read_with(cx, |shell, _| {
            let form = shell.areas.conversation.answering.as_ref().unwrap();
            assert_eq!(form.step, 2);
            assert_eq!(
                form.drafts.get(&"question_1".parse().unwrap()),
                Some(&Draft::Picked(vec!["a".into()]))
            );
        });
    });
}
