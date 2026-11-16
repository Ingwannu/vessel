//! Identified callbacks use the same engine/chrome and retain original sources.
use super::*;

struct Identified {
    field: Option<SearchField>,
    retired: Option<SearchField>,
    scheme: Scheme,
    seen: Seen,
    observed: Vec<(FieldId, String)>,
    accepted: Vec<String>,
    rejected: Vec<FieldId>,
    rail: Pixels,
}
impl Identified {
    fn changed(&mut self, source: FieldId, text: String, cx: &mut Context<'_, Self>) {
        self.observed.push((source, text.clone()));
        if self
            .field
            .as_ref()
            .is_some_and(|field| field.id() == source)
        {
            self.accepted.push(text);
        } else {
            self.rejected.push(source);
        }
        cx.notify();
    }
    fn field(window: &mut Window, cx: &mut Context<'_, Self>) -> SearchField {
        SearchField::with_source("Filter", Self::changed, window, cx)
    }
}
impl Render for Identified {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let seen = Rc::clone(&self.seen);
        div().size_full().p(px(24.0)).child(
            div()
                .w(self.rail)
                .flex()
                .on_children_prepainted(move |bounds, _, _| *seen.borrow_mut() = bounds)
                .children(
                    self.field.as_ref().map(|field| {
                        field.element("identified-filter", &Tokens::of(self.scheme), cx)
                    }),
                ),
        )
    }
}
fn open_probe(
    scheme: Scheme,
    seen: Seen,
    window: &mut Window,
    cx: &mut Context<'_, Identified>,
) -> Identified {
    Identified {
        field: Some(Identified::field(window, cx)),
        retired: None,
        scheme,
        seen,
        observed: Vec::new(),
        accepted: Vec::new(),
        rejected: Vec::new(),
        rail: px(800.0),
    }
}

#[gpui::test]
fn identified_search_keeps_search_clear_chrome_and_real_focus(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap());
    for scheme in [Scheme::Light, Scheme::Dark] {
        let seen = Seen::default();
        let (view, cx) = cx.add_window_view({
            let seen = Rc::clone(&seen);
            move |window, cx| open_probe(scheme, seen, window, cx)
        });
        cx.update(|window, _| window.activate_window());
        cx.run_until_parked();
        let wide = *seen.borrow().first().unwrap();
        assert_eq!(wide.size.width, px(420.0));
        assert_eq!(wide.size.height, px(34.0));
        assert!(cx.debug_bounds("search-field-clear").is_none());
        view.update_in(cx, |this, window, cx| {
            let field = this.field.as_ref().unwrap();
            let source = field.id();
            assert_eq!(field.clone().id(), source);
            field.set_text("retained", window, cx);
            assert_eq!(field.text(cx), "retained");
            field.set_placeholder("검색", window, cx);
            assert_eq!(field.id(), source);
            field.clear(window, cx);
            field.focus(window, cx);
        });
        cx.run_until_parked();
        view.read_with(cx, |this, _| assert!(this.observed.is_empty()));
        cx.simulate_input("한글");
        cx.run_until_parked();
        let clear = cx.debug_bounds("search-field-clear").unwrap();
        assert_eq!(clear.size.width, px(30.0));
        assert_eq!(clear.size.height, px(30.0));
        cx.simulate_click(clear.center(), Modifiers::none());
        cx.run_until_parked();
        view.update_in(cx, |this, window, cx| {
            let field = this.field.as_ref().unwrap();
            assert_eq!(field.text(cx), "");
            assert!(field.editor.focus_handle(cx).is_focused(window));
            assert_eq!(
                this.observed,
                [
                    (field.id(), "한".to_owned()),
                    (field.id(), "한글".to_owned()),
                    (field.id(), String::new()),
                ]
            );
            assert_eq!(this.accepted, ["한", "한글", ""]);
            assert!(this.rejected.is_empty());
            this.rail = px(260.0);
            cx.notify();
        });
        cx.run_until_parked();
        assert!(cx.debug_bounds("search-field-clear").is_none());
        let narrow = *seen.borrow().first().unwrap();
        assert_eq!(narrow.size.width, px(260.0));
        assert_eq!(narrow.size.height, px(34.0));
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        view.read_with(cx, |this, _| assert_eq!(this.observed.len(), 3));
    }
}

#[gpui::test]
fn queued_old_search_change_keeps_its_source_after_close_and_reopen(cx: &mut TestAppContext) {
    cx.update(|cx| crate::init(cx, SchemeChoice::Fixed(Scheme::Dark)).unwrap());
    let (view, cx) =
        cx.add_window_view(|window, cx| open_probe(Scheme::Dark, Seen::default(), window, cx));
    cx.update(|window, _| window.activate_window());
    view.update_in(cx, |this, window, cx| {
        let old = this.field.take().unwrap();
        let old_source = old.id();
        this.retired = Some(old.clone());
        // A queued engine event survives its original opening. Retain the old
        // engine to prove rejection, rather than an automatically dropped event.
        cx.defer_in(window, move |_, window, cx| match &old.editor.kind {
            Kind::Line(state) => {
                state.update(cx, |state, cx| state.replace_all("obsolete", window, cx));
            }
            Kind::Lines(_, _) => panic!("a search field must use the one-line engine"),
        });
        this.field = Some(Identified::field(window, cx));
        let current = this.field.as_ref().unwrap();
        assert_ne!(current.id(), old_source);
        current.focus(window, cx);
        cx.notify();
    });
    cx.run_until_parked();
    view.read_with(cx, |this, cx| {
        let old_source = this.retired.as_ref().unwrap().id();
        assert_eq!(this.observed, [(old_source, "obsolete".to_owned())]);
        assert_eq!(this.rejected, [old_source]);
        assert!(this.accepted.is_empty());
        assert_eq!(this.field.as_ref().unwrap().text(cx), "");
    });
    cx.simulate_input("fresh");
    cx.run_until_parked();
    view.read_with(cx, |this, _| {
        assert_eq!(this.accepted, ["f", "fr", "fre", "fres", "fresh"]);
        assert_eq!(this.rejected.len(), 1);
        assert_eq!(
            this.observed.last().unwrap().0,
            this.field.as_ref().unwrap().id()
        );
    });
}
