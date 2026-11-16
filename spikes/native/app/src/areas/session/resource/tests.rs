//! The actual field callback refreshes only its retained resident claim.
use super::*;
use crate::{
    areas::{conversation::ConversationArea, layout::residents::SessionResources},
    shell::language_tests,
};
use gpui::{AppContext as _, Entity, IntoElement, ParentElement as _, Render, TestAppContext};
use std::{cell::RefCell, rc::Rc, time::Duration};
use vessel_app_kit::{Scheme, Tokens};
use vessel_app_view::layout::{
    AgentRole, Layout, LayoutKey, LayoutMode, OpenIntent, PaneId, Placement, Position, Target,
};
use vessel_app_view::residents::{
    PanelClaim, ResidentToken, SessionAddress, SessionContext, Visibility,
};
use vessel_proto::{NodeId, SessionId, WorkspaceId};

type Observed = Rc<RefCell<Vec<(SessionId, PaneId, String)>>>;
struct Harness {
    shell: Entity<Shell>,
    field: TextField,
    a: ResidentToken,
    b: ResidentToken,
    claim_a: PanelClaim,
    claim_b: PanelClaim,
    side: PaneId,
    observed: Observed,
}
impl Render for Harness {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        gpui::div().child(self.field.element(&Tokens::of(Scheme::Dark), false, cx))
    }
}
fn claim(layout: &mut Layout, key: &LayoutKey) -> PanelClaim {
    PanelClaim {
        workspace: key.clone(),
        tab: layout
            .open(
                Target::NewTab,
                OpenIntent::New,
                Placement::Focused,
                AgentRole::Unknown,
                LayoutMode::SplitPanes,
            )
            .unwrap(),
    }
}
fn build(
    context: &SessionContext,
    observed: Observed,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> SessionResources {
    SessionResources {
        session: SessionView::new_scoped(
            context.clone(),
            move |shell, current, text, _, cx| {
                shell
                    .areas
                    .session_residents
                    .resources_mut(current)
                    .unwrap()
                    .session
                    .resume_error = Some(text.clone());
                observed
                    .borrow_mut()
                    .push((current.resident.address.session, current.pane, text));
                cx.notify();
            },
            window,
            cx,
        ),
        conversation: ConversationArea::new(cx),
    }
}

fn new_harness(window: &mut Window, cx: &mut Context<'_, Harness>) -> Harness {
    // Shell remains the callback owner; the harness only draws its exact
    // retained field. No global editor is swapped or used as an adapter.
    let shell = cx.new(|cx| Shell::without_io(window, cx));
    let observed: Observed = Rc::default();
    let (field, a, b, claim_a, claim_b, side) = shell.update(cx, |shell, cx| {
        let node = NodeId::random();
        let key = LayoutKey::Workspace {
            node,
            workspace: WorkspaceId::random(),
        };
        let mut layout = Layout::default();
        let claim_a = claim(&mut layout, &key);
        let claim_b = claim(&mut layout, &key);
        let side = layout
            .split(PaneId::MAIN, Position::Right, LayoutMode::SplitPanes)
            .unwrap();
        let residents = &mut shell.areas.session_residents;
        let a = residents
            .claim_session(
                SessionAddress {
                    node,
                    session: SessionId::random(),
                },
                &claim_a,
                PaneId::MAIN,
                |context| build(context, Rc::clone(&observed), window, cx),
            )
            .unwrap();
        let b = residents
            .claim_session(
                SessionAddress {
                    node,
                    session: SessionId::random(),
                },
                &claim_b,
                PaneId::MAIN,
                |context| build(context, Rc::clone(&observed), window, cx),
            )
            .unwrap();
        residents
            .present(a, &claim_a, PaneId::MAIN, Visibility::Visible)
            .unwrap();
        residents.focus(a, &claim_a).unwrap();
        let current = residents.context(a, &claim_a).unwrap();
        let field = residents
            .resources(&current)
            .unwrap()
            .session
            .composer
            .clone();
        field.set_text("draft A", window, cx);
        let other = residents.context(b, &claim_b).unwrap();
        residents
            .resources(&other)
            .unwrap()
            .session
            .composer
            .set_text("draft B", window, cx);
        (field, a, b, claim_a, claim_b, side)
    });
    Harness {
        shell,
        field,
        a,
        b,
        claim_a,
        claim_b,
        side,
        observed,
    }
}

fn first_submit(harness: &Entity<Harness>, window: &mut gpui::VisualTestContext) {
    window.run_until_parked();
    harness.update_in(window, |harness, window, cx| {
        window.activate_window();
        harness.field.focus(window, cx);
    });
    window.simulate_keystrokes("enter");
    window.run_until_parked();
    harness.read_with(window, |harness, cx| {
        assert_eq!(
            &*harness.observed.borrow(),
            &[(
                harness.a.address.session,
                PaneId::MAIN,
                "draft A".to_owned()
            )]
        );
        assert_eq!(
            harness.field.text(cx),
            "draft A",
            "constructor never clears on submit"
        );
        let shell = harness.shell.read(cx);
        let current = shell
            .areas
            .session_residents
            .context(harness.a, &harness.claim_a)
            .unwrap();
        assert_eq!(
            shell
                .areas
                .session_residents
                .resources(&current)
                .unwrap()
                .session
                .composer
                .id(),
            harness.field.id()
        );
    });
}

fn release_original(harness: &Entity<Harness>, window: &mut gpui::VisualTestContext) {
    harness.update_in(window, |harness, _, cx| {
        harness.shell.update(cx, |shell, _| {
            let residents = &mut shell.areas.session_residents;
            residents
                .present(
                    harness.a,
                    &harness.claim_a,
                    harness.side,
                    Visibility::RetainedHidden,
                )
                .unwrap();
            residents
                .release_claim(harness.a, &harness.claim_a, Duration::ZERO)
                .unwrap();
        });
    });
    window.simulate_keystrokes("enter");
    window.run_until_parked();
    harness.read_with(window, |harness, _| {
        assert_eq!(harness.observed.borrow().len(), 2);
    });
}

#[gpui::test]
fn scoped_submit_survives_moves_and_refuses_hidden_or_released_claims(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (harness, window) = cx.add_window_view(new_harness);
    first_submit(&harness, window);
    window.simulate_keystrokes("left shift-left");
    window.run_until_parked();
    let retained = harness.read_with(window, |harness, cx| {
        (harness.field.focus_handle(cx), harness.field.selection(cx))
    });
    harness.update_in(window, |harness, _, cx| {
        harness.shell.update(cx, |shell, _| {
            let residents = &mut shell.areas.session_residents;
            residents
                .present(
                    harness.a,
                    &harness.claim_a,
                    PaneId::MAIN,
                    Visibility::RetainedHidden,
                )
                .unwrap();
            residents
                .present(
                    harness.b,
                    &harness.claim_b,
                    PaneId::MAIN,
                    Visibility::Visible,
                )
                .unwrap();
            residents.focus(harness.b, &harness.claim_b).unwrap();
        });
    });
    // Deliberately deliver a stale visible-frame Enter before its old field can
    // lose platform focus. It must not submit into either resident.
    window.simulate_keystrokes("enter");
    window.run_until_parked();
    harness.read_with(window, |harness, cx| {
        assert_eq!(harness.observed.borrow().len(), 1);
        assert_eq!(harness.field.text(cx), "draft A");
        assert_eq!(harness.field.selection(cx), retained.1);
    });
    harness.update_in(window, |harness, _, cx| {
        harness.shell.update(cx, |shell, _| {
            let residents = &mut shell.areas.session_residents;
            residents
                .present(
                    harness.a,
                    &harness.claim_a,
                    harness.side,
                    Visibility::Visible,
                )
                .unwrap();
            residents.focus(harness.a, &harness.claim_a).unwrap();
        });
    });
    window.simulate_keystrokes("enter");
    window.run_until_parked();
    harness.read_with(window, |harness, cx| {
        assert_eq!(harness.observed.borrow().len(), 2);
        assert_eq!(
            harness.observed.borrow()[1].1,
            harness.side,
            "refresh current placement of same claim"
        );
        assert_eq!(harness.field.focus_handle(cx), retained.0);
        assert_eq!(harness.field.selection(cx), retained.1);
        let shell = harness.shell.read(cx);
        let context = shell
            .areas
            .session_residents
            .context(harness.b, &harness.claim_b)
            .unwrap();
        assert_eq!(
            shell
                .areas
                .session_residents
                .resources(&context)
                .unwrap()
                .session
                .composer
                .text(cx),
            "draft B"
        );
    });
    release_original(&harness, window);
}
