//! Real user-bubble drawing uses the shared literal text engine in both find states.
use super::{Paint, Prompt, person};
use crate::areas::conversation::{
    ConversationArea, Seat, find::Marking, look::Look, origin::Above, pacing,
};
use gpui::{Context, Render, TestAppContext, Window};
use std::cell::Cell;
use std::rc::Rc;
use vessel_app_kit::ui::{IntoElement as _, ParentElement as _, Styled as _, div, px};
use vessel_app_kit::{Scheme, Tokens, VirtualList};
use vessel_app_view::Lang;
use vessel_app_view::fork::Headers;
use vessel_app_view::localization::{ENGLISH, Translator};
use vessel_app_view::timeline::Match;

const SOURCE: &str = "<spoken-input>\nfix **literal** <example>한글</example> fix\n</spoken-input>";

struct Probe {
    area: ConversationArea,
    prompt: Prompt,
    headers: Headers,
    list: VirtualList,
    query: Option<&'static str>,
    paints: Rc<Cell<usize>>,
}

impl Render for Probe {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        let tokens = Tokens::of(Scheme::Dark);
        let find = self.query.map(|query| Marking {
            query,
            current: Some(Match { entry: 0, nth: 1 }),
            list: &self.list,
            clear_top: px(0.0),
        });
        let paint = Paint {
            catalog: Translator::get(ENGLISH).unwrap(),
            words: Lang::En.conversation(),
            tokens: &tokens,
            look: Look::of(&tokens),
            find,
            seat: Seat::SESSION,
            entries: &[],
            agent: None,
            rewinds: None,
            forks: None,
            fork_headers: &self.headers,
            above: Above::Nothing,
        };
        let body = person(
            &paint,
            0,
            &self.prompt,
            None,
            pacing::history(&self.area),
            window,
            cx,
        );
        self.paints.set(self.paints.get().saturating_add(1));
        div()
            .w_full()
            .h_full()
            .child(gpui_base::TextSelectionLayer)
            .child(body)
            .into_any_element()
    }
}

#[gpui::test]
fn user_bubble_draws_literal_voice_words_with_find_and_after_find_closes(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    let paints = Rc::new(Cell::new(0));
    let observed = Rc::clone(&paints);
    let (view, cx) = cx.add_window_view(move |_, cx| Probe {
        area: ConversationArea::new(cx),
        prompt: Prompt {
            text: SOURCE.to_owned(),
            replayed: false,
            turn: 1,
            attachments: Vec::new(),
        },
        headers: Headers::default(),
        list: VirtualList::new(false),
        query: Some("fix"),
        paints,
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(observed.get() > 0);
    let with_find = observed.get();
    view.update_in(cx, |probe, _, cx| {
        assert_eq!(probe.prompt.text, SOURCE);
        probe.query = None;
        cx.notify();
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(observed.get() > with_find);
    view.read_with(cx, |probe, _| assert_eq!(probe.prompt.text, SOURCE));
}
