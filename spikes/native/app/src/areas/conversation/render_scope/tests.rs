//! Real owner-minted claims and the exact Window scope wrapper used by rows/sheets.
use super::RenderScope;
use gpui::{Context, Render, TestAppContext, Window};
use std::cell::RefCell;
use std::rc::Rc;
use vessel_app_kit::ui::{IntoElement as _, ParentElement as _, div};
use vessel_app_view::layout::{
    AgentRole, Layout, LayoutKey, LayoutMode, OpenIntent, PaneId, Placement, Target,
};
use vessel_app_view::residents::{
    PanelClaim, Residents, SessionAddress, SessionContext, Visibility,
};
use vessel_proto::{NodeId, SessionId, WorkspaceId};

fn contexts() -> (SessionContext, SessionContext) {
    let node = NodeId::random();
    let workspace = LayoutKey::Workspace {
        node,
        workspace: WorkspaceId::random(),
    };
    let mut layout = Layout::default();
    let mut claim = || PanelClaim {
        workspace: workspace.clone(),
        tab: layout
            .open(
                Target::NewTab,
                OpenIntent::New,
                Placement::Focused,
                AgentRole::Unknown,
                LayoutMode::SplitPanes,
            )
            .unwrap(),
    };
    let (a, b) = (claim(), claim());
    let mut residents = Residents::new();
    let address = SessionAddress {
        node,
        session: SessionId::random(),
    };
    let token = residents.claim(address, a.clone(), PaneId::MAIN).unwrap();
    assert_eq!(
        residents.claim(address, b.clone(), PaneId::MAIN).unwrap(),
        token
    );
    let first = residents.context(token, &a).unwrap();
    residents
        .present(token, &a, PaneId::EXPLORER, Visibility::RetainedHidden)
        .unwrap();
    let moved = residents.context(token, &a).unwrap();
    assert_eq!(
        RenderScope::admitted(&first).element(),
        RenderScope::admitted(&moved).element()
    );
    (moved, residents.context(token, &b).unwrap())
}

#[test]
fn captured_claim_origin_and_sheet_paths_are_distinct_without_reallocating_identity() {
    let (a, b) = contexts();
    let first = RenderScope::admitted(&a);
    let second = RenderScope::admitted(&b);
    assert_ne!(first.element(), second.element());
    assert_eq!(first.element(), RenderScope::admitted(&a).element());
    assert_ne!(first.element(), first.origin().element());
    assert_ne!(first.origin().element(), first.origin().origin().element());
    assert_ne!(first.sheet().element(), first.element());
    assert_ne!(first.sheet().element(), second.sheet().element());
}

type Observed = Rc<RefCell<Vec<Vec<(u64, u64)>>>>;
struct Probe {
    scopes: Vec<(RenderScope, u64)>,
    observed: Observed,
}
impl Render for Probe {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        let values = self
            .scopes
            .iter()
            .map(|(scope, initial)| {
                let row = scope.with(window, |window| {
                    let state = window.use_keyed_state("retained-control", cx, |_, _| *initial);
                    *state.read(cx)
                });
                let sheet = scope.sheet().with(window, |window| {
                    let state = window
                        .use_keyed_state("retained-control", cx, |_, _| initial.saturating_mul(10));
                    *state.read(cx)
                });
                (row, sheet)
            })
            .collect::<Vec<_>>();
        self.observed.borrow_mut().push(values);
        div().child("scope probe").into_any_element()
    }
}

#[gpui::test]
fn actual_row_and_portal_scope_wrappers_keep_warm_control_state_per_claim(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let (a, b) = contexts();
    let observed = Rc::new(RefCell::new(Vec::new()));
    let results = Rc::clone(&observed);
    let (view, cx) = cx.add_window_view(move |_, _| Probe {
        scopes: vec![
            (RenderScope::admitted(&a), 7),
            (RenderScope::admitted(&b), 11),
        ],
        observed,
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(results.borrow().last().unwrap(), &vec![(7, 70), (11, 110)]);
    view.update_in(cx, |probe, _, cx| {
        probe.scopes.swap(0, 1);
        for (_, initial) in &mut probe.scopes {
            *initial = 99;
        }
        cx.notify();
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(results.borrow().last().unwrap(), &vec![(11, 110), (7, 70)]);
}
