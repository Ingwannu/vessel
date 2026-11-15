use super::*;
use gpui::{Context, ParentElement as _, Render, Styled as _, TestAppContext, div};
use std::fmt::Write as _;
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

const IMAGE_URIS: [&str; 4] = [
    "data:image/svg+xml,%3Csvg%20xmlns='http://www.w3.org/2000/svg'%3E%3C/svg%3E",
    "file:///PRIVATE-IMAGE-CANARY.png",
    "http://example.invalid/remote.png",
    "relative/image.png",
];

struct Probe {
    guards: MarkdownGuards,
    text: SharedString,
}
impl Render for Probe {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        div().w_full().child(markdown_guarded(
            "guarded-document",
            self.text.clone(),
            self.guards.clone(),
        ))
    }
}

#[gpui::test]
fn every_refused_uri_uses_the_same_inert_fixed_error_loader(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::init(cx, crate::SchemeChoice::Fixed(crate::Scheme::Dark)).expect("kit");
    });
    let (view, cx) = cx.add_window_view(|_, _| Probe {
        text: "plain".into(),
        guards: MarkdownGuards {
            on_link: Arc::new(|_, _, _, _| panic!("no opener")),
            images: ImagePolicy::RefuseAll,
        },
    });
    view.update_in(cx, |_view, window, cx| {
        let mut previous = None;
        for text in IMAGE_URIS {
            let uri: SharedUri = text.to_owned().into();
            let ImageSource::Custom(loader) = ImagePolicy::RefuseAll.source(&uri) else {
                panic!("inert source only");
            };
            if let Some(previous) = &previous {
                assert!(Arc::ptr_eq(previous, &loader));
            }
            match loader(window, cx) {
                Some(Err(ImageCacheError::Asset(message))) => assert_eq!(message.as_ref(), REFUSAL),
                Some(Ok(_)) | Some(Err(_)) | None => panic!("fixed refusal required"),
            }
            previous = Some(loader);
        }
    });
}

#[gpui::test]
fn actual_markdown_routes_all_image_kinds_through_the_authoritative_hook_without_fetch(
    cx: &mut TestAppContext,
) {
    let requests = Arc::new(AtomicUsize::new(0));
    let fetched = Arc::clone(&requests);
    cx.update(|cx| {
        crate::init(cx, crate::SchemeChoice::Fixed(crate::Scheme::Dark)).expect("kit");
        cx.set_http_client(gpui::http_client::FakeHttpClient::create(move |_request| {
            let _prior = fetched.fetch_add(1, Ordering::SeqCst);
            std::future::ready(Ok(gpui::http_client::Response::new(
                gpui::http_client::AsyncBody::default(),
            )))
        }));
    });
    let resolved = Arc::new(Mutex::new(Vec::new()));
    let observed = Arc::clone(&resolved);
    let mut text = String::new();
    for uri in IMAGE_URIS {
        writeln!(text, "![image]({uri})\n").expect("fixture Markdown");
    }
    let (_view, cx) = cx.add_window_view(move |_, _| Probe {
        text: text.into(),
        guards: MarkdownGuards {
            on_link: Arc::new(|_, _, _, _| panic!("no opener")),
            images: ImagePolicy::Resolve(Arc::new(move |uri| {
                observed
                    .lock()
                    .expect("observed image hook")
                    .push(uri.to_string());
                ImagePolicy::RefuseAll.source(uri)
            })),
        },
    });
    cx.run_until_parked();
    let seen = resolved.lock().expect("resolved images");
    for uri in IMAGE_URIS {
        assert!(
            seen.iter().any(|seen| seen == uri),
            "every image uses the hook"
        );
    }
    assert_eq!(requests.load(Ordering::SeqCst), 0);
}
