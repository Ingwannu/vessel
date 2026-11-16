use super::*;
use crate::shell::language_tests;
use gpui::{
    App, Entity, InputEvent as _, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, Render,
    TestAppContext, VisualTestContext, point, px,
};
use std::cell::Cell;
use vessel_app_kit::Scheme;
use vessel_app_kit::sidebar_callout::Actions;
use vessel_app_kit::ui::{IntoElement, ParentElement as _, Styled as _, div};

fn registration(source: SourceId, priority: i32, on_dismiss: Rc<Cell<usize>>) -> Registration {
    Registration {
        source,
        priority: Priority::new(priority),
        dismissal: None,
        draw: Rc::new(|shell, _| Presentation {
            content: Content {
                title: Some(shell.words.sidebar().header_title().to_owned().into()),
                description: None,
                icon: None,
                actions: Actions::None,
                dismiss: None,
            },
            variant: Variant::Default,
        }),
        dismissible: true,
        on_dismiss: Some(Rc::new(move |_, _, _| {
            on_dismiss.set(on_dismiss.get().saturating_add(1));
        })),
    }
}

struct Probe(Entity<Shell>, Rc<Cell<usize>>);
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.1.set(self.1.get().saturating_add(1));
        self.0.update(cx, |shell, cx| {
            div()
                .w(px(320.0))
                .children(render(shell, &Tokens::of(Scheme::Dark), cx))
        })
    }
}

fn setup(cx: &mut TestAppContext) -> (Entity<Shell>, Rc<Cell<usize>>, &mut VisualTestContext) {
    let (probe, cx) = cx.add_window_view(|window, cx| {
        use gpui::AppContext as _;
        let shell = cx.new(|cx| Shell::without_io(window, cx));
        cx.observe(&shell, |_, _, cx| cx.notify()).detach();
        Probe(shell, Rc::new(Cell::new(0)))
    });
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    let renders = probe.read_with(cx, |probe, _| Rc::clone(&probe.1));
    (shell, renders, cx)
}

#[gpui::test]
fn sidebar_callout_pointer_dismissal_uses_the_existing_preference_writer_and_callback_once(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, _, cx) = setup(cx);
    let dismissed = Rc::new(Cell::new(0));
    let source = SourceId::try_from("fixture".to_owned()).unwrap();
    let key = DismissalKey::try_from("fixture:release".to_owned()).unwrap();
    shell.update_in(cx, |shell, _, cx| {
        let mut entry = registration(source, 0, Rc::clone(&dismissed));
        entry.dismissal = Some(key.clone());
        let _token = shell.sidebar_callout_show(entry, cx).unwrap();
    });
    cx.run_until_parked();
    cx.simulate_click(point(px(297.0), px(20.0)), Modifiers::none());
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        assert!(shell.areas.sidebar.callouts.admission.active().is_none());
        assert_eq!(
            shell.prefs.settings.sidebar_callout_dismissals.keys(),
            &[key]
        );
    });
    assert_eq!(dismissed.get(), 1);
    cx.simulate_click(point(px(297.0), px(20.0)), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(dismissed.get(), 1);
}

#[gpui::test]
fn sidebar_callout_loaded_dismissals_and_cleanup_share_one_admission_state(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, _, cx) = setup(cx);
    let dismissed = Rc::new(Cell::new(0));
    shell.update_in(cx, |shell, _, cx| {
        let hidden = SourceId::try_from("hidden".to_owned()).unwrap();
        let update = SourceId::try_from("update".to_owned()).unwrap();
        let key = DismissalKey::try_from("saved:release".to_owned()).unwrap();
        shell.prefs.settings.sidebar_callout_dismissals =
            Dismissals::try_from(vec![key.clone()]).unwrap();
        let mut entry = registration(hidden, 100, Rc::clone(&dismissed));
        entry.dismissal = Some(key);
        let _hidden_token = shell.sidebar_callout_show(entry, cx).unwrap();
        let old = shell
            .sidebar_callout_show(registration(update.clone(), 0, Rc::clone(&dismissed)), cx)
            .unwrap();
        let new = shell
            .sidebar_callout_show(registration(update.clone(), 1, Rc::clone(&dismissed)), cx)
            .unwrap();
        shell.sidebar_callout_unregister(&update, old, cx);
        assert_eq!(
            shell
                .areas
                .sidebar
                .callouts
                .admission
                .active()
                .unwrap()
                .id(),
            &update
        );
        shell.sidebar_callout_unregister(&update, new, cx);
        assert!(shell.areas.sidebar.callouts.admission.active().is_none());
        assert_eq!(dismissed.get(), 0);
    });
}

#[gpui::test]
fn sidebar_callout_nondismissible_provider_cannot_bypass_shared_admission(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, _, cx) = setup(cx);
    let dismissed = Rc::new(Cell::new(0));
    shell.update_in(cx, |shell, _, cx| {
        let mut entry = registration(
            SourceId::try_from("fixed".to_owned()).unwrap(),
            0,
            Rc::clone(&dismissed),
        );
        entry.dismissible = false;
        entry.draw = Rc::new({
            let dismissed = Rc::clone(&dismissed);
            move |_, _| Presentation {
                content: Content {
                    title: Some("Fixed notice".into()),
                    description: None,
                    icon: None,
                    actions: Actions::None,
                    dismiss: Some(Dismiss {
                        press: Rc::new({
                            let dismissed = Rc::clone(&dismissed);
                            move |_, _, _| dismissed.set(dismissed.get().saturating_add(1))
                        }),
                    }),
                },
                variant: Variant::Error,
            }
        });
        let _token = shell.sidebar_callout_show(entry, cx).unwrap();
    });
    cx.run_until_parked();
    cx.simulate_click(point(px(297.0), px(20.0)), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(dismissed.get(), 0);
    shell.read_with(cx, |shell, _| {
        assert!(shell.areas.sidebar.callouts.admission.active().is_some());
    });
}

fn click_retained_frame(window: &mut Window, cx: &mut App) {
    let position = point(px(297.0), px(20.0));
    let _down = window.dispatch_event(
        MouseDownEvent {
            position,
            button: MouseButton::Left,
            modifiers: Modifiers::none(),
            click_count: 1,
            first_mouse: false,
        }
        .to_platform_input(),
        cx,
    );
    let _up = window.dispatch_event(
        MouseUpEvent {
            position,
            button: MouseButton::Left,
            modifiers: Modifiers::none(),
            click_count: 1,
        }
        .to_platform_input(),
        cx,
    );
}

#[gpui::test]
fn sidebar_callout_old_render_cannot_dismiss_replacement_before_redraw(cx: &mut TestAppContext) {
    language_tests::init(cx);
    for dismissible in [false, true] {
        let (shell, renders, cx) = setup(cx);
        let dismissed = Rc::new(Cell::new(0));
        let source = SourceId::try_from("replace".to_owned()).unwrap();
        shell.update_in(cx, |shell, _, cx| {
            let _token = shell
                .sidebar_callout_show(registration(source.clone(), 0, Rc::clone(&dismissed)), cx)
                .unwrap();
        });
        cx.run_until_parked();
        let before = renders.get();
        cx.update(|window, cx| {
            shell.update(cx, |shell, cx| {
                let mut entry = registration(source.clone(), 0, Rc::clone(&dismissed));
                entry.dismissal = Some(DismissalKey::try_from("new-key".to_owned()).unwrap());
                entry.dismissible = dismissible;
                let _token = shell.sidebar_callout_show(entry, cx).unwrap();
            });
            click_retained_frame(window, cx);
            assert_eq!(
                renders.get(),
                before,
                "A's actual rendered input must be retained"
            );
            shell.read_with(cx, |shell, _| {
                assert_eq!(
                    shell
                        .areas
                        .sidebar
                        .callouts
                        .admission
                        .active()
                        .unwrap()
                        .id(),
                    &source
                );
                assert!(shell.prefs.settings.sidebar_callout_dismissals.is_empty());
            });
            assert_eq!(dismissed.get(), 0);
        });
        cx.run_until_parked();
        cx.simulate_click(point(px(297.0), px(20.0)), Modifiers::none());
        cx.run_until_parked();
        assert_eq!(dismissed.get(), usize::from(dismissible));
        shell.read_with(cx, |shell, _| {
            assert_eq!(
                shell.areas.sidebar.callouts.admission.active().is_none(),
                dismissible
            );
        });
    }
}

#[gpui::test]
fn sidebar_callout_canonical_apply_and_rollback_reconcile_without_recreating_or_dismissing(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, _, cx) = setup(cx);
    let dismissed = Rc::new(Cell::new(0));
    shell.update_in(cx, |shell, _, cx| {
        let source = SourceId::try_from("durable".to_owned()).unwrap();
        let key = DismissalKey::try_from("canonical:one".to_owned()).unwrap();
        let mut entry = registration(source.clone(), 1, Rc::clone(&dismissed));
        entry.dismissal = Some(key.clone());
        let token = shell.sidebar_callout_show(entry, cx).unwrap();
        let ephemeral = SourceId::try_from("ephemeral".to_owned()).unwrap();
        let _other = shell
            .sidebar_callout_show(
                registration(ephemeral.clone(), 0, Rc::clone(&dismissed)),
                cx,
            )
            .unwrap();
        shell.prefs.settings.sidebar_callout_dismissals = Dismissals::try_from(vec![key]).unwrap();
        preferences_changed(shell, cx);
        assert_eq!(
            shell
                .areas
                .sidebar
                .callouts
                .admission
                .active()
                .unwrap()
                .id(),
            &ephemeral
        );
        let canonical = shell.prefs.settings.sidebar_callout_dismissals.clone();
        preferences_changed(shell, cx);
        assert_eq!(shell.prefs.settings.sidebar_callout_dismissals, canonical);
        shell.prefs.settings.sidebar_callout_dismissals = Dismissals::default();
        preferences_changed(shell, cx);
        let restored = shell.areas.sidebar.callouts.admission.active().unwrap();
        assert_eq!(restored.id(), &source);
        assert_eq!(
            restored.token(),
            token,
            "rollback retains the original provider payload"
        );
        shell.sidebar_callout_unregister(&source, token, cx);
        preferences_changed(shell, cx);
        assert_eq!(
            shell
                .areas
                .sidebar
                .callouts
                .admission
                .active()
                .unwrap()
                .id(),
            &ephemeral
        );
        assert_eq!(dismissed.get(), 0);
    });
}
