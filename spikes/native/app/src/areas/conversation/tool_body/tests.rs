#[test]
fn borrowed_request_sections_preserve_structural_parent_identity() {
    use vessel_app_kit::ui::ElementId;
    let request = ElementId::NamedInteger("permit".into(), 1);
    let text_alias = ElementId::Name("permit-1".into());
    assert_eq!(request.to_string(), text_alias.to_string());
    assert_ne!(
        super::child_id(&request, "section-0"),
        super::child_id(&text_alias, "section-0")
    );
    assert_ne!(
        super::child_id(&request, "section-0"),
        super::child_id(&request, "failure")
    );
    assert_eq!(
        super::child_id(&request, "section-0"),
        super::child_id(&request, "section-0")
    );
}

use crate::areas::conversation::{Seat, entries::Paint, look::Look, origin::Above, tool_detail};
use gpui::{Context, Render, TestAppContext, Window};
use std::cell::Cell;
use std::rc::Rc;
use vessel_app_kit::ui::{IntoElement as _, ParentElement as _, Styled as _, div};
use vessel_app_kit::{Scheme, Tokens};
use vessel_app_view::Lang;
use vessel_app_view::fork::Headers;
use vessel_app_view::localization::{ENGLISH, Translator};
use vessel_proto::timeline::{ShellDetail, ToolCallDetail, UnknownToolDetail};

struct ProposedBody {
    detail: ToolCallDetail,
    state: tool_detail::DetailContentState,
    headers: Headers,
    painted: Rc<Cell<usize>>,
}
impl Render for ProposedBody {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let paint = Paint {
            catalog: Translator::get(ENGLISH).unwrap(),
            words: Lang::En.conversation(),
            tokens: &tokens,
            look: Look::of(&tokens),
            find: None,
            seat: Seat::SESSION,
            entries: &[],
            agent: None,
            rewinds: None,
            forks: None,
            fork_headers: &self.headers,
            above: Above::Nothing,
        };
        let body = tool_detail::borrowed_body(
            &paint,
            &vessel_app_kit::ui::ElementId::NamedInteger("proposed-body".into(), 1),
            tool_detail::BorrowedDetail {
                detail: &self.detail,
                state: self.state,
                failure: None,
            },
            window,
            cx,
        );
        self.painted.set(self.painted.get().saturating_add(1));
        div().w_full().h_full().child(body).into_any_element()
    }
}

#[gpui::test]
fn proposed_material_draws_through_the_borrowed_api_without_tool_or_log_objects(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let painted = Rc::new(Cell::new(0));
    let observed = Rc::clone(&painted);
    let (view, cx) = cx.add_window_view(move |_, _| ProposedBody {
        detail: ToolCallDetail::Shell(ShellDetail {
            command: "echo proposed".to_owned(),
            cwd: None,
            output: None,
            exit_code: None,
        }),
        state: tool_detail::DetailContentState::Ready,
        headers: Headers::default(),
        painted,
    });
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    let ready_paints = observed.get();
    assert!(ready_paints > 0);
    view.update_in(cx, |probe, _, cx| {
        probe.detail = ToolCallDetail::Unknown(UnknownToolDetail {
            input: None,
            output: None,
        });
        probe.state = tool_detail::DetailContentState::Loading;
        cx.notify();
    });
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    assert!(observed.get() > ready_paints);
}
