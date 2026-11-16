//! Public Markdown helper, fake platform activation and actual refusal resolver.
use super::{denied_image, unavailable};
use gpui::{Context, Modifiers, MouseButton, Render, TestAppContext, Window, point, px};
use std::cell::Cell;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use vessel_app_kit::chat_markdown::actions::{Actions, CopyStyle};
use vessel_app_kit::chat_markdown::{Options, chat_markdown_with};
use vessel_app_kit::ui::{IntoElement as _, ParentElement as _, Styled as _, div};
use vessel_app_kit::{Scheme, Tokens, version_of};
use vessel_app_view::Lang;

std::thread_local! { static DENIALS: Cell<usize> = const { Cell::new(0) }; }
pub(super) fn denied() {
    DENIALS.with(|count| count.set(count.get().saturating_add(1)));
}

struct Probe {
    source: String,
    allow_probe: bool,
    calls: Arc<AtomicUsize>,
}
impl Render for Probe {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> impl gpui::IntoElement {
        let actions = if self.allow_probe {
            let calls = Arc::clone(&self.calls);
            Actions::new(move |_, _, _| {
                let _previous = calls.fetch_add(1, Ordering::SeqCst);
            })
        } else {
            unavailable(Lang::En.options().unavailable)
        };
        div()
            .w(px(300.0))
            .child(chat_markdown_with(
                "unavailable-policy-probe",
                &self.source,
                version_of(&self.source),
                Options {
                    marks: None,
                    actions: Some(actions),
                    copy: CopyStyle::Plain,
                },
                &Tokens::of(Scheme::Dark),
                window,
                cx,
            ))
            .into_any_element()
    }
}

#[gpui::test]
fn actual_public_helper_suppresses_file_relative_and_external_activation(cx: &mut TestAppContext) {
    crate::shell::language_tests::init(cx);
    for href in [
        "file:///vessel-denied-link.txt",
        "src/lib.rs",
        "https://example.invalid",
    ] {
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&calls);
        let source = format!("[open]({href})");
        let (probe, window) = cx.add_window_view(move |_, _| Probe {
            source,
            allow_probe: true,
            calls,
        });
        window.run_until_parked();
        window.simulate_click(point(px(8.0), px(8.0)), Modifiers::default());
        assert_eq!(
            observed.load(Ordering::SeqCst),
            1,
            "positive control must hit the rendered link"
        );
        assert_eq!(window.opened_url(), None);
        probe.update_in(window, |probe, _, cx| {
            probe.allow_probe = false;
            cx.notify();
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        window.simulate_click(point(px(8.0), px(8.0)), Modifiers::default());
        window.simulate_mouse_down(
            point(px(8.0), px(8.0)),
            MouseButton::Middle,
            Modifiers::default(),
        );
        window.simulate_mouse_up(
            point(px(8.0), px(8.0)),
            MouseButton::Middle,
            Modifiers::default(),
        );
        assert_eq!(observed.load(Ordering::SeqCst), 1);
        assert_eq!(
            window.opened_url(),
            None,
            "the TestPlatform records any default URL attempt"
        );
    }
}

#[gpui::test]
fn all_image_sources_use_the_same_immediate_refusal_without_pixels_or_uri_fallback(
    cx: &mut TestAppContext,
) {
    crate::shell::language_tests::init(cx);
    for uri in [
        "file:///vessel-denied-image.png",
        "relative/image.png",
        "https://example.invalid/image.png",
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=",
    ] {
        DENIALS.with(|count| count.set(0));
        let source = format!("![image]({uri})");
        let (_, window) = cx.add_window_view(move |_, _| Probe {
            source,
            allow_probe: false,
            calls: Arc::new(AtomicUsize::new(0)),
        });
        window.run_until_parked();
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(
            DENIALS.with(Cell::get) > 0,
            "the real public helper must invoke the denial resolver"
        );
        assert_eq!(window.opened_url(), None);
        window.update(|window, cx| {
            let gpui::ImageSource::Custom(load) = denied_image(Lang::Ko.options().unavailable.into()) else { panic!("refusal is not a URI/resource/pixel source") };
            assert!(matches!(load(window, cx), Some(Err(gpui::ImageCacheError::Asset(message))) if message.as_ref() == Lang::Ko.options().unavailable));
        });
    }
}
