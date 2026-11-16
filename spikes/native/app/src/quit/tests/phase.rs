//! Reserve inside a Shell callback; activate only after all window borrows end.

use super::*;
use gpui::{AppContext as _, Entity, EntityId, Global, IntoElement as _, Render, Window, div};

#[derive(Default)]
struct Probe(Vec<(EntityId, bool)>);
impl Global for Probe {}

fn apply_probe(_shell: &mut Shell, cx: &mut gpui::Context<'_, Shell>) {
    let active = is_finalizing(cx);
    let entity = cx.entity().entity_id();
    cx.global_mut::<Probe>().0.push((entity, active));
}

fn registered(cx: &mut TestAppContext) -> Entity<Shell> {
    let window = cx.add_window(|window, cx| {
        let shell = Shell::without_io(window, cx);
        crate::host_windows::register(&cx.entity(), window, cx);
        shell
    });
    cx.update(|cx| window.entity(cx).unwrap())
}

#[gpui::test]
fn every_owner_is_applied_synchronously_after_reservation_and_before_any_effect(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let first = registered(cx);
    let second = registered(cx);
    cx.update(|cx| {
        cx.set_global(Probe::default());
        set_document_phase_applier(apply_probe, cx);
    });
    first.update(cx, |_shell, cx| {
        assert!(begin(cx));
        assert!(
            !begin(cx),
            "queued requests coalesce without reborrowing a Shell"
        );
        assert!(cx.global::<Probe>().0.is_empty());
    });
    cx.update(|cx| {
        assert!(activate(cx));
        let expected = [first.entity_id(), second.entity_id()];
        let events = &cx.global::<Probe>().0;
        assert_eq!(events.len(), 2);
        assert!(expected.iter().all(|id| events.contains(&(*id, true))));
        // This is the first possible effect turn; no await occurred after the
        // same-registry owner callbacks. Domain engine behavior remains C5's.
        assert!(is_finalizing(cx));
        finish(cx);
        let events = &cx.global::<Probe>().0;
        assert_eq!(events.len(), 4);
        assert!(expected.iter().all(|id| events.contains(&(*id, false))));
        assert!(!is_finalizing(cx));
    });
}

struct Unknown;
impl Render for Unknown {
    fn render(
        &mut self,
        _: &mut Window,
        _: &mut gpui::Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        div().into_any_element()
    }
}

#[gpui::test]
fn an_unknown_window_refuses_activation_without_partial_owner_admission(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let shell = registered(cx);
    cx.update(|cx| {
        cx.set_global(Probe::default());
        set_document_phase_applier(apply_probe, cx);
    });
    shell.update(cx, |_shell, cx| {
        assert!(begin(cx));
    });
    cx.update(|cx| {
        let unknown = cx
            .open_window(gpui::WindowOptions::default(), |_, cx| cx.new(|_| Unknown))
            .unwrap();
        assert!(!activate(cx));
        assert!(cx.global::<Probe>().0.is_empty());
        assert!(!is_finalizing(cx));
        unknown
            .update(cx, |_, window, _| window.remove_window())
            .unwrap();
        assert!(begin(cx));
        assert!(activate(cx));
        finish(cx);
    });
}
