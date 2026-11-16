//! Retain a rendered production dialog, replace its intent without a frame,
//! then deliver each of its old dismissal paths. None may close the new one.
use super::*;
use gpui::{
    AppContext as _, Entity, InputEvent as _, KeyDownEvent, Keystroke, Modifiers, MouseButton,
    MouseUpEvent, Render,
};
use std::{cell::Cell, rc::Rc};
use vessel_app_kit::ui::{IntoElement, MouseDownEvent, Point, div, point};

struct Probe {
    shell: Entity<Shell>,
    renders: Rc<Cell<usize>>,
}
impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.renders.set(self.renders.get().saturating_add(1));
        self.shell.update(cx, |shell, cx| {
            dialog(shell, &Tokens::of(vessel_app_kit::Scheme::Dark), window, cx)
                .unwrap_or_else(|| div().into_any_element())
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum Dismissal {
    Escape,
    Scrim,
    Glyph,
}
#[derive(Debug, Clone, Copy)]
enum Replacement {
    Intent,
    Connection,
}

fn click_without_a_frame(
    cx: &mut gpui::VisualTestContext,
    at: Point<gpui::Pixels>,
    renders: &Cell<usize>,
) -> usize {
    // Keep down/up on the same rendered tree. Running the executor between
    // them would legitimately render the replacement and test its new glyph.
    cx.update(|window, cx| {
        let _down = window.dispatch_event(
            MouseDownEvent {
                position: at,
                button: MouseButton::Left,
                modifiers: Modifiers::default(),
                click_count: 1,
                first_mouse: false,
            }
            .to_platform_input(),
            cx,
        );
        let _up = window.dispatch_event(
            MouseUpEvent {
                position: at,
                button: MouseButton::Left,
                modifiers: Modifiers::default(),
                click_count: 1,
            }
            .to_platform_input(),
            cx,
        );
        renders.get()
    })
}

#[gpui::test]
fn old_escape_scrim_and_glyph_cannot_dismiss_another_intent_or_connection(cx: &mut TestAppContext) {
    cx.update(|cx| {
        vessel_app_kit::init(
            cx,
            vessel_app_kit::SchemeChoice::Fixed(vessel_app_kit::Scheme::Dark),
        )
        .unwrap();
    });
    for replacement in [Replacement::Intent, Replacement::Connection] {
        for path in [Dismissal::Escape, Dismissal::Scrim, Dismissal::Glyph] {
            let mut epochs = ConnectionEpochs::default();
            let first = epochs
                .admit(MachineKey::from_book("fixture"), NodeId::random())
                .unwrap();
            let next = match replacement {
                Replacement::Intent => first.clone(),
                Replacement::Connection => {
                    epochs.admit(first.machine().clone(), first.node()).unwrap()
                }
            };
            let renders = Rc::new(Cell::new(0));
            let (probe, cx) = cx.add_window_view({
                let first = first.clone();
                let renders = Rc::clone(&renders);
                move |window, cx| {
                    let shell = cx.new(|cx| {
                        let mut shell = Shell::without_io(window, cx);
                        let page = &mut shell.areas.settings.devices.page;
                        page.target(Target::Online(first.machine().clone()));
                        let _change = page.bind(Some(first.clone()));
                        grant(page, &first);
                        page.listing = Listing::Ready(vec![device()]);
                        page.choose_bound(&first, device().id).unwrap();
                        shell.areas.settings.devices.dialog.focus(window, cx);
                        shell
                    });
                    Probe { shell, renders }
                }
            });
            cx.run_until_parked();
            if matches!(path, Dismissal::Escape) {
                // GPUI redraws dirty key dispatch (window.rs:6053). Enter
                // keyboard modality before retaining the old rendered tree.
                cx.simulate_keystrokes("left");
            }
            let glyph = cx.debug_bounds("dialog-close").unwrap().center();
            let drawn = renders.get();
            let expected = probe.update_in(cx, |probe, _, cx| {
                probe.shell.update(cx, |shell, _cx| {
                    let page = &mut shell.areas.settings.devices.page;
                    let old_intent = page.revoke_intent().unwrap();
                    page.cancel_revoke(&first, old_intent);
                    let _change = page.bind(Some(next.clone()));
                    grant(page, &next);
                    page.listing = Listing::Ready(vec![device()]);
                    page.choose_bound(&next, device().id).unwrap();
                    page.revoke_intent().unwrap()
                })
            });
            assert_eq!(renders.get(), drawn, "the replacement has not been drawn");
            let handled_frame = match path {
                Dismissal::Escape => cx.update(|window, cx| {
                    let _escape = window.dispatch_event(
                        KeyDownEvent {
                            keystroke: Keystroke::parse("escape").unwrap(),
                            is_held: false,
                            prefer_character_input: false,
                        }
                        .to_platform_input(),
                        cx,
                    );
                    renders.get()
                }),
                Dismissal::Scrim => {
                    click_without_a_frame(cx, point(px(0.0), px(0.0)), renders.as_ref())
                }
                Dismissal::Glyph => click_without_a_frame(cx, glyph, renders.as_ref()),
            };
            // cx.update may flush a normal redraw after the input callback.
            // Count inside dispatch, while retaining the actual old tree.
            assert_eq!(handled_frame, drawn, "{replacement:?}/{path:?}: input tree");
            probe.read_with(cx, |probe, cx| {
                probe.shell.read_with(cx, |shell, _cx| {
                    let page = &shell.areas.settings.devices.page;
                    assert_eq!(page.context(), Some(&next));
                    assert_eq!(
                        page.revoke_intent(),
                        Some(expected),
                        "{replacement:?}/{path:?}"
                    );
                    assert!(matches!(page.revoking, Revoking::Confirm(_)));
                });
            });
        }
    }
}
