use super::*;
use crate::shell::language_tests;
use gpui::{Entity, Modifiers, Render, TestAppContext, VisualTestContext, point, px};
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

struct Probe(Entity<Shell>);
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.0.update(cx, |shell, cx| {
            div()
                .w(px(320.0))
                .children(render(shell, &Tokens::of(Scheme::Dark), cx))
        })
    }
}

fn setup(cx: &mut TestAppContext) -> (Entity<Shell>, &mut VisualTestContext) {
    let (probe, cx) = cx.add_window_view(|window, cx| {
        use gpui::AppContext as _;
        let shell = cx.new(|cx| Shell::without_io(window, cx));
        cx.observe(&shell, |_, _, cx| cx.notify()).detach();
        Probe(shell)
    });
    let shell = probe.read_with(cx, |probe, _| probe.0.clone());
    (shell, cx)
}

#[gpui::test]
fn sidebar_callout_pointer_dismissal_uses_the_existing_preference_writer_and_callback_once(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (shell, cx) = setup(cx);
    let dismissed = Rc::new(Cell::new(0));
    let source = SourceId::new("fixture");
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
    let (shell, cx) = setup(cx);
    let dismissed = Rc::new(Cell::new(0));
    shell.update_in(cx, |shell, _, cx| {
        let hidden = SourceId::new("hidden");
        let update = SourceId::new("update");
        let key = DismissalKey::try_from("saved:release".to_owned()).unwrap();
        shell.prefs.settings.sidebar_callout_dismissals =
            Dismissals::try_from(vec![key.clone()]).unwrap();
        let mut entry = registration(hidden, 100, Rc::clone(&dismissed));
        entry.dismissal = Some(key);
        let _hidden_token = shell.sidebar_callout_show(entry, cx).unwrap();
        let old = shell
            .sidebar_callout_show(registration(update, 0, Rc::clone(&dismissed)), cx)
            .unwrap();
        let new = shell
            .sidebar_callout_show(registration(update, 1, Rc::clone(&dismissed)), cx)
            .unwrap();
        shell.sidebar_callout_unregister(update, old, cx);
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
        shell.sidebar_callout_unregister(update, new, cx);
        assert!(shell.areas.sidebar.callouts.admission.active().is_none());
        assert_eq!(dismissed.get(), 0);
    });
}
