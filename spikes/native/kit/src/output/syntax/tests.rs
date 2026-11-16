use super::highlights;
use crate::code_editor::{CodeStyles, SyntaxKind};
use crate::{Scheme, Tokens};

#[test]
fn syntax_and_changed_ranges_merge_without_overlaps_and_recolor_from_current_scheme() {
    let syntax = vec![(0..6, Some(SyntaxKind::Keyword))];
    let changed = 2..4;
    let dark = CodeStyles::of(Scheme::Dark);
    let light = CodeStyles::of(Scheme::Light);
    let background = Tokens::of(Scheme::Dark).st_ready;
    let runs = highlights(
        std::slice::from_ref(&changed),
        &syntax,
        Some(background),
        &dark,
    );
    assert_eq!(
        runs.iter()
            .map(|(range, _)| range.clone())
            .collect::<Vec<_>>(),
        [0..2, 2..4, 4..6]
    );
    assert!(runs.windows(2).all(|pair| pair[0].0.end <= pair[1].0.start));
    assert!(
        runs.iter()
            .all(|(_, style)| style.color == dark.highlight(SyntaxKind::Keyword).color)
    );
    assert_eq!(runs[1].1.background_color, Some(background));
    assert!(runs[0].1.background_color.is_none());
    let recolored = highlights(
        std::slice::from_ref(&changed),
        &syntax,
        Some(background),
        &light,
    );
    assert_eq!(
        recolored[1].1.color,
        light.highlight(SyntaxKind::Keyword).color
    );
    assert_ne!(runs[1].1.color, recolored[1].1.color);
    assert_eq!(runs[1].1.background_color, recolored[1].1.background_color);
}

#[test]
fn absent_semantic_runs_preserve_background_only_behavior() {
    let styles = CodeStyles::of(Scheme::Dark);
    let background = Tokens::of(Scheme::Dark).st_ready;
    let ranges = 0..3;
    let runs = highlights(
        std::slice::from_ref(&ranges),
        &[],
        Some(background),
        &styles,
    );
    assert_eq!(runs.len(), 1);
    assert!(runs[0].1.color.is_none());
    assert_eq!(runs[0].1.background_color, Some(background));
    assert!(highlights(std::slice::from_ref(&ranges), &[], None, &styles).is_empty());
}

struct Probe {
    scheme: Scheme,
    builds: std::rc::Rc<std::cell::Cell<usize>>,
    version: u64,
}

impl gpui::Render for Probe {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        use gpui::{IntoElement as _, ParentElement as _, Styled as _};
        let tokens = Tokens::of(self.scheme);
        let box_ = crate::output::syntax_lines_box(
            "semantic-output-probe",
            self.version,
            || {
                self.builds.set(self.builds.get().saturating_add(1));
                crate::output::SyntaxLines {
                    output: crate::output::Lines::text("fn 보드() {}").into(),
                    syntax: vec![vec![(0..2, Some(SyntaxKind::Keyword))]],
                }
            },
            crate::output::Start::Top,
            &tokens,
            window,
            cx,
        );
        gpui::div().w_full().h_full().child(box_).into_any_element()
    }
}

#[gpui::test]
fn actual_virtualized_output_reuses_semantic_rows_across_theme_changes(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(|cx| {
        crate::init(cx, crate::SchemeChoice::Fixed(Scheme::Dark)).unwrap();
    });
    let builds = std::rc::Rc::new(std::cell::Cell::new(0));
    let observed = std::rc::Rc::clone(&builds);
    let (view, cx) = cx.add_window_view(move |_, _| Probe {
        scheme: Scheme::Dark,
        builds,
        version: 1,
    });
    for scheme in [Scheme::Dark, Scheme::Light, Scheme::Dark] {
        view.update_in(cx, |probe, _, cx| {
            probe.scheme = scheme;
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }
    assert_eq!(observed.get(), 1);
    view.update_in(cx, |probe, _, cx| {
        probe.version = 2;
        cx.notify();
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(observed.get(), 2);
}

#[test]
fn plain_file_tokens_use_the_same_independent_palette_as_the_existing_preview() {
    for scheme in [Scheme::Dark, Scheme::Light] {
        assert_eq!(
            CodeStyles::of(scheme).highlight(SyntaxKind::Variable).color,
            Some(crate::code::CodeTokens::of(scheme).plain)
        );
    }
}
