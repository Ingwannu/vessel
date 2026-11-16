//! Real retained line/textarea callbacks must not edit the next form's engines.
use super::{Input, Inputs};
use crate::shell::{Shell, language_tests};
use gpui::{AppContext as _, Entity, Render, TestAppContext, VisualTestContext};
use vessel_app_kit::{
    Scheme, TextField, Tokens,
    ui::{Context, IntoElement, Window, div},
};
use vessel_app_view::schedules::editor::cadence;
use vessel_proto::automation::TimeZoneName;

fn field(shell: &Shell, input: Input) -> Option<&TextField> {
    let inputs = shell.areas.schedules.ui.editor.inputs.as_ref()?;
    Some(match input {
        Input::Name => &inputs.name,
        Input::Prompt => &inputs.prompt,
        Input::Limit => &inputs.limit,
        Input::Rule => &inputs.rule,
    })
}
fn start(shell: &mut Shell, window: &mut Window, cx: &mut Context<'_, Shell>) {
    shell.route = crate::routes::Route::Schedules;
    shell.areas.schedules.ui.editor.mounted = true;
    let initial = cadence::initial(TimeZoneName::utc());
    assert!(initial.is_ok());
    let Ok(initial) = initial else {
        return;
    };
    let opened = shell
        .areas
        .schedules
        .data
        .editor
        .create(initial, TimeZoneName::utc());
    assert!(matches!(&opened, Ok(Some(_))));
    let Ok(Some(visit)) = opened else {
        return;
    };
    shell.areas.schedules.ui.editor.inputs = Inputs::new(shell, visit, window, cx);
    assert!(shell.areas.schedules.ui.editor.inputs.is_some());
}
struct Probe {
    shell: Entity<Shell>,
    input: Input,
    retained: Option<TextField>,
}
impl Render for Probe {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.shell.update(cx, |shell, cx| {
            let field = self.retained.as_ref().or_else(|| field(shell, self.input));
            field.map_or_else(
                || div().into_any_element(),
                |field| field.element(&Tokens::of(Scheme::Dark), false, cx),
            )
        })
    }
}
fn name(shell: &Shell) -> Option<&str> {
    shell
        .areas
        .schedules
        .data
        .editor
        .draft()?
        .fields()
        .map(vessel_app_view::schedules::editor::fields::Fields::name)
}
fn prompt(shell: &Shell) -> Option<&str> {
    shell
        .areas
        .schedules
        .data
        .editor
        .draft()?
        .fields()
        .map(vessel_app_view::schedules::editor::fields::Fields::prompt)
}
#[gpui::test]
fn actual_retained_line_and_multiline_sources_cannot_edit_the_next_form(cx: &mut TestAppContext) {
    language_tests::init(cx);
    for input in [Input::Name, Input::Prompt] {
        let (view, cx) = cx.add_window_view(|window, cx| {
            let shell = cx.new(|cx| {
                let mut shell = Shell::without_io(window, cx);
                start(&mut shell, window, cx);
                shell
            });
            let retained = field(shell.read(cx), input).cloned();
            assert!(retained.is_some());
            Probe {
                shell,
                input,
                retained,
            }
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            view.update(cx, |this, cx| {
                this.shell.update(cx, |shell, cx| {
                    let old = field(shell, input);
                    assert!(old.is_some());
                    if let Some(old) = old {
                        old.focus(window, cx);
                    }
                });
            });
        });
        cx.simulate_input("first");
        assert_text(&view, cx, "first");
        cx.update(|window, cx| {
            view.update(cx, |this, cx| {
                this.shell.update(cx, |shell, cx| {
                    let visit = shell.areas.schedules.data.editor.visit();
                    assert!(visit.is_some());
                    if let Some(visit) = visit {
                        assert!(shell.areas.schedules.data.editor.close(visit));
                    }
                    start(shell, window, cx);
                });
            });
        });
        // The original real engine remains focused and mounted in Probe. Its
        // callback still carries A's FieldId, while Shell now retains B's id.
        cx.simulate_input("late");
        assert_text(&view, cx, "");
        cx.update(|window, cx| {
            view.update(cx, |this, cx| {
                this.retained = None;
                this.shell.update(cx, |shell, cx| {
                    let current = field(shell, input);
                    assert!(current.is_some());
                    if let Some(current) = current {
                        current.focus(window, cx);
                    }
                });
                cx.notify();
            });
        });
        cx.run_until_parked();
        cx.simulate_input("current");
        assert_text(&view, cx, "current");
        holds_while_away(&view, cx);
    }
}

fn assert_text(view: &Entity<Probe>, cx: &mut VisualTestContext, expected: &str) {
    view.read_with(cx, |this, cx| {
        let shell = this.shell.read(cx);
        let actual = match this.input {
            Input::Name => name(shell),
            Input::Prompt => prompt(shell),
            Input::Limit | Input::Rule => None,
        };
        assert_eq!(actual, Some(expected));
    });
}
fn holds_while_away(view: &Entity<Probe>, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        view.update(cx, |this, cx| {
            this.shell.update(cx, |shell, cx| {
                super::super::leaving(shell, window, cx);
                shell.route = crate::routes::Route::Draft;
                let _changed = shell.schedules_changed(cx);
            });
        });
    });
    cx.simulate_input("hidden");
    assert_text(view, cx, "current");
    view.read_with(cx, |this, cx| {
        let current = field(this.shell.read(cx), this.input);
        assert!(current.is_some());
        if let Some(current) = current {
            assert_eq!(current.text(cx), "current");
        }
    });
    cx.update(|window, cx| {
        view.update(cx, |this, cx| {
            this.shell.update(cx, |shell, cx| {
                shell.route = crate::routes::Route::Schedules;
                super::super::arrive(shell, window, cx);
                let current = field(shell, this.input);
                assert!(current.is_some());
                if let Some(current) = current {
                    current.focus(window, cx);
                }
            });
        });
    });
    cx.simulate_input("resumed");
    assert_text(view, cx, "currentresumed");
}
