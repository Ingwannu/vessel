//! Actual cached output keeps truncation metadata without rerunning the script.
use super::diff_output;
use gpui::{Context, Render, TestAppContext, Window};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use vessel_app_kit::ui::{IntoElement as _, ParentElement as _, Styled as _, div};
use vessel_app_kit::{Scheme, Tokens, version_of};
use vessel_app_view::timeline::{MAX_LINES, diff_lines};

struct Probe {
    text: String,
    builds: Rc<Cell<usize>>,
    cuts: Rc<RefCell<Vec<bool>>>,
}

impl Render for Probe {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let (output, cut) = diff_output(
            "diff-cache-probe".into(),
            version_of(&self.text),
            || {
                self.builds.set(self.builds.get().saturating_add(1));
                diff_lines(Some("before"), &self.text)
            },
            &tokens,
            window,
            cx,
        );
        self.cuts.borrow_mut().push(cut);
        div().w_full().h_full().child(output).into_any_element()
    }
}

fn draw(cx: &mut gpui::VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

#[gpui::test]
fn warm_diff_draws_do_not_repeat_edit_work_and_keep_the_cut_caption(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let builds = Rc::new(Cell::new(0));
    let cuts = Rc::new(RefCell::new(Vec::new()));
    let observed_builds = Rc::clone(&builds);
    let observed_cuts = Rc::clone(&cuts);
    let (view, cx) = cx.add_window_view(move |_, _| Probe {
        text: "after".to_owned(),
        builds,
        cuts,
    });
    for _ in 0..10 {
        view.update_in(cx, |_, _, cx| cx.notify());
        draw(cx);
    }
    assert_eq!(observed_builds.get(), 1);
    assert!(observed_cuts.borrow().len() >= 10);
    assert!(observed_cuts.borrow().iter().all(|cut| !cut));

    view.update_in(cx, |probe, _, cx| {
        probe.text = "line\n".repeat(MAX_LINES.saturating_add(1));
        cx.notify();
    });
    draw(cx);
    let from = observed_cuts.borrow().len();
    for _ in 0..10 {
        view.update_in(cx, |_, _, cx| cx.notify());
        draw(cx);
    }
    assert_eq!(observed_builds.get(), 2);
    let flags = observed_cuts.borrow();
    assert!(flags.last().copied().unwrap());
    let warm = flags.get(from..).unwrap();
    assert!(warm.len() >= 10);
    assert!(warm.iter().all(|cut| *cut));
    drop(flags);

    view.update_in(cx, |probe, _, cx| {
        probe.text = "short again".to_owned();
        cx.notify();
    });
    draw(cx);
    assert_eq!(observed_builds.get(), 3);
    assert!(!observed_cuts.borrow().last().copied().unwrap());
}
