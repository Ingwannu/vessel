//! Pointer activation through the same plain-image and mixed inline-flow renderers.

use std::sync::{Arc, Mutex};

use gpui::{
    Context, ImageSource, IntoElement, Modifiers, ParentElement as _, Render, RenderImage,
    SharedUri, Styled as _, TestAppContext, Window, div, point, px,
};

use super::{Actions, CopyStyle};
use crate::chat_markdown::{Options, chat_markdown_with};
use crate::{Scheme, Tokens};

struct Probe {
    source: &'static str,
    activate: bool,
    decoded: Arc<RenderImage>,
    clicks: Arc<Mutex<Vec<SharedUri>>>,
    links: Arc<Mutex<Vec<String>>>,
}

impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let decoded = Arc::clone(&self.decoded);
        let links = Arc::clone(&self.links);
        let mut actions =
            Actions::new(move |uri, _, _| links.lock().unwrap().push(uri.as_str().to_owned()))
                .with_images(move |_| ImageSource::Render(Arc::clone(&decoded)));
        if self.activate {
            let clicks = Arc::clone(&self.clicks);
            actions = actions.on_image_click(move |uri, _, _| {
                clicks.lock().unwrap().push(uri.0);
            });
        }
        let tokens = Tokens::of(Scheme::Dark);
        div().w(px(300.0)).child(chat_markdown_with(
            "image-activation-probe",
            self.source,
            0,
            Options {
                marks: None,
                actions: Some(actions),
                copy: CopyStyle::MarkdownCodeAware,
            },
            &tokens,
            window,
            cx,
        ))
    }
}

#[gpui::test]
fn unlinked_images_activate_in_plain_and_inline_flow_without_opening_a_url(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_base::init);
    for source in [
        "![preview](relative/image.png)",
        "![preview](relative/image.png) suffix",
        "[![preview](relative/image.png)](https://example.com)",
    ] {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let clicks = Arc::clone(&captured);
        let links = Arc::new(Mutex::new(Vec::new()));
        let links_seen = Arc::clone(&links);
        let (_, window) = cx.add_window_view(move |_, _| Probe {
            source,
            activate: true,
            decoded: Arc::new(RenderImage::new(vec![image::Frame::new(
                image::RgbaImage::from_pixel(64, 32, image::Rgba([255, 255, 255, 255])),
            )])),
            clicks,
            links,
        });
        window.run_until_parked();
        window.simulate_click(point(px(8.0), px(8.0)), Modifiers::default());
        let seen = captured.lock().unwrap();
        assert_eq!(seen.len(), 1, "{source}");
        assert_eq!(seen[0].as_ref(), "relative/image.png");
        assert!(
            links_seen.lock().unwrap().is_empty(),
            "one click cannot also activate the enclosing link"
        );
        assert_eq!(
            window.opened_url(),
            None,
            "image activation must not navigate the browser"
        );
    }
}

#[gpui::test]
fn absence_of_image_handler_preserves_unlinked_and_linked_behavior(cx: &mut TestAppContext) {
    cx.update(gpui_base::init);
    for (source, linked) in [
        ("![preview](relative/image.png)", false),
        (
            "[![preview](relative/image.png)](https://example.com)",
            true,
        ),
    ] {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let clicks = Arc::clone(&captured);
        let links = Arc::new(Mutex::new(Vec::new()));
        let links_seen = Arc::clone(&links);
        let (_, window) = cx.add_window_view(move |_, _| Probe {
            source,
            activate: false,
            decoded: Arc::new(RenderImage::new(vec![image::Frame::new(
                image::RgbaImage::from_pixel(64, 32, image::Rgba([255, 255, 255, 255])),
            )])),
            clicks,
            links,
        });
        window.run_until_parked();
        window.simulate_click(point(px(8.0), px(8.0)), Modifiers::default());
        assert!(captured.lock().unwrap().is_empty());
        assert_eq!(links_seen.lock().unwrap().len(), usize::from(linked));
        assert_eq!(window.opened_url(), None);
    }
}
