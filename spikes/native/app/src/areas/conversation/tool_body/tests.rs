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
use std::cell::RefCell;
use std::rc::Rc;
use vessel_app_kit::output::{Lines, Tone};
use vessel_app_kit::ui::{IntoElement as _, ParentElement as _, Styled as _, div};
use vessel_app_kit::{Scheme, Tokens};
use vessel_app_view::Lang;
use vessel_app_view::fork::Headers;
use vessel_app_view::localization::{ENGLISH, Translator};
use vessel_proto::timeline::{ShellDetail, ToolCallDetail, UnknownToolDetail};

enum Observation {
    Requested(gpui::ElementId),
    Admitted(gpui::ElementId, Lines),
}
type Admissions = Rc<RefCell<Vec<Observation>>>;
std::thread_local! {
    static WATCH: RefCell<Option<Admissions>> = const { RefCell::new(None) };
}

// Observe the exact model admitted by the real Kit factory, not a second cache
// or a public inspection bypass. Unchanged warm draws never invoke this hook.
pub(super) fn admitted(id: &gpui::ElementId, lines: &Lines) {
    WATCH.with_borrow(|watch| {
        if let Some(watch) = watch {
            watch
                .borrow_mut()
                .push(Observation::Admitted(id.clone(), lines.clone()));
        }
    });
}

pub(super) fn requested(id: &gpui::ElementId) {
    WATCH.with_borrow(|watch| {
        if let Some(watch) = watch {
            watch.borrow_mut().push(Observation::Requested(id.clone()));
        }
    });
}

struct Watch {
    models: Admissions,
    previous: Option<Admissions>,
}
impl Watch {
    fn begin() -> Self {
        let models = Rc::new(RefCell::new(Vec::new()));
        let previous = WATCH.replace(Some(Rc::clone(&models)));
        Self { models, previous }
    }

    fn count(&self) -> usize {
        self.models
            .borrow()
            .iter()
            .filter(|event| matches!(event, Observation::Admitted(..)))
            .count()
    }

    fn current(&self) -> Lines {
        let observations = self.models.borrow();
        let current = observations
            .iter()
            .rev()
            .find_map(|event| match event {
                Observation::Requested(id) => Some(id),
                Observation::Admitted(..) => None,
            })
            .unwrap();
        observations
            .iter()
            .rev()
            .find_map(|event| match event {
                Observation::Admitted(id, lines) if id == current => Some(lines.clone()),
                Observation::Admitted(..) | Observation::Requested(_) => None,
            })
            .expect("current output must have admitted a model")
    }
}
impl Drop for Watch {
    fn drop(&mut self) {
        WATCH.replace(self.previous.take());
    }
}

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

fn detail(value: serde_json::Value) -> ToolCallDetail {
    serde_json::from_value(value).unwrap()
}

fn draw(cx: &mut gpui::VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

const PATCH: &str = "@@ -1 +1 @@\n-old\n+new";

fn assert_rows(model: &Lines, rows: &[(&str, Tone, Option<usize>)], copy: Option<&str>) {
    let actual: Vec<_> = model
        .lines
        .iter()
        .map(|line| (line.text.as_ref(), line.tone, line.number))
        .collect();
    assert_eq!(actual, rows);
    assert_eq!(model.copy.as_deref(), copy);
}

#[gpui::test]
fn equal_bytes_change_format_in_both_directions_through_the_retained_borrowed_api(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    let watch = Watch::begin();
    let (view, cx) = cx.add_window_view(|_, _| ProposedBody {
        detail: ToolCallDetail::Unknown(UnknownToolDetail {
            input: None,
            output: None,
        }),
        state: tool_detail::DetailContentState::Ready,
        headers: Headers::default(),
        painted: Rc::new(Cell::new(0)),
    });
    for patch in [false, true, false, true] {
        let next = if patch {
            detail(serde_json::json!({"type":"edit","file_path":"a","unified_diff":PATCH}))
        } else {
            detail(serde_json::json!({"type":"search","query":"q","content":PATCH}))
        };
        view.update_in(cx, |probe, _, cx| {
            probe.detail = next;
            cx.notify();
        });
        draw(cx);
        let made = watch.current();
        if patch {
            assert_rows(
                &made,
                &[
                    ("@@ -1 +1 @@", Tone::Plain, None),
                    ("old", Tone::Removed, Some(1)),
                    ("new", Tone::Added, Some(1)),
                ],
                Some(PATCH),
            );
        } else {
            assert_rows(
                &made,
                &[
                    ("@@ -1 +1 @@", Tone::Plain, None),
                    ("-old", Tone::Plain, None),
                    ("+new", Tone::Plain, None),
                ],
                Some(PATCH),
            );
        }
        let admitted = watch.count();
        for _ in 0..4 {
            view.update_in(cx, |_, _, cx| cx.notify());
            draw(cx);
        }
        assert_eq!(
            watch.count(),
            admitted,
            "warm draw must reuse the same projection"
        );
    }
}

#[gpui::test]
fn empty_patch_uses_reported_replacement_or_deletion_in_the_real_body(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let watch = Watch::begin();
    let (view, cx) = cx.add_window_view(|_, _| ProposedBody {
        detail: detail(serde_json::json!({"type":"edit","file_path":"a","old_string":"old","new_string":"new","unified_diff":""})),
        state: tool_detail::DetailContentState::Ready,
        headers: Headers::default(),
        painted: Rc::new(Cell::new(0)),
    });
    draw(cx);
    assert_rows(
        &watch.current(),
        &[
            ("old", Tone::Removed, Some(1)),
            ("new", Tone::Added, Some(1)),
        ],
        None,
    );
    view.update_in(cx, |probe, _, cx| {
        probe.detail = detail(serde_json::json!({"type":"edit","file_path":"a","old_string":"old","new_string":"","unified_diff":""}));
        cx.notify();
    });
    draw(cx);
    assert_rows(&watch.current(), &[("old", Tone::Removed, Some(1))], None);
    let before = watch.count();
    view.update_in(cx, |probe, _, cx| {
        probe.detail = detail(
            serde_json::json!({"type":"edit","file_path":"a","old_string":"old","unified_diff":""}),
        );
        cx.notify();
    });
    draw(cx);
    assert_eq!(
        watch.count(),
        before,
        "missing replacement has no fabricated diff"
    );
}
