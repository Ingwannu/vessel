//! Images opened from a tray or message share one modal and restore focus.
//! Viewport geometry belongs to view; this area owns only window input.

mod decode;
mod draw;
/// Register the modal with the root without adding feature logic there.
pub(crate) fn overlay(
    shell: &mut Shell,
    tokens: &vessel_app_kit::Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
    started: Option<std::time::Instant>,
) -> vessel_app_kit::ui::Div {
    use vessel_app_kit::ui::ParentElement as _;
    vessel_app_kit::ui::div()
        .children(crate::areas::composer_shell::fullscreen(
            shell, tokens, window, cx,
        ))
        .children(super::context::preview(shell, tokens, window, cx))
        .children(draw::render(shell, tokens, window, cx))
        .child(vessel_app_kit::frame_log::drawn(started))
}

use crate::shell::Shell;
use gpui::{
    FocusHandle, ImageSource, MouseMoveEvent, MouseUpEvent, PinchEvent, ScrollDelta, ScrollEvent,
};
use vessel_app_kit::ui::{App, Context, Pixels, Point, Window};
use vessel_app_view::composer::sent::{Picture, Sent};
use vessel_app_view::composer::viewer::{Position, Viewport};
use vessel_proto::AttachmentRef;

/// Four explicit actions share one tooltip set and one placement per trigger.
const ACTIONS: usize = 4;

/// The one open image, including a remote reference while it loads.
#[derive(Clone)]
enum Preview {
    Local(ImageSource),
    Sent { sent: Sent, file: AttachmentRef },
}

/// Whether the pointer gesture has moved since its press.
#[derive(Debug, Clone, Copy)]
enum Gesture {
    Pressed(Position),
    Dragged(Position),
}

/// Window-only state; bytes remain owned by the bounded picture cache.
pub(super) struct Lightbox {
    preview: Option<Preview>,
    local_lease: Option<std::sync::Arc<vessel_app_view::composer::local::LocalFile>>,
    focus: FocusHandle,
    previous: Option<FocusHandle>,
    viewport: Option<Viewport>,
    gesture: Option<Gesture>,
    tips: vessel_app_kit::Tooltips,
    places: [vessel_app_kit::tooltip::Place; ACTIONS],
}

impl std::fmt::Debug for Lightbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Lightbox")
            .field("open", &self.preview.is_some())
            .finish_non_exhaustive()
    }
}

impl Lightbox {
    #[cfg(test)]
    pub(super) fn is_open(&self) -> bool {
        self.preview.is_some()
    }

    pub(super) fn new(cx: &mut App) -> Self {
        Self {
            preview: None,
            local_lease: None,
            focus: cx.focus_handle(),
            previous: None,
            viewport: None,
            gesture: None,
            tips: vessel_app_kit::Tooltips::new(),
            places: Default::default(),
        }
    }

    fn open(&mut self, preview: Preview, window: &mut Window, cx: &mut App) {
        if self.preview.is_none() {
            self.previous = window.focused(cx);
        }
        self.local_lease = None;
        self.preview = Some(preview);
        self.viewport = None;
        self.gesture = None;
        self.focus.focus(window, cx);
    }
}

impl Shell {
    /// Other areas can reuse the modal after decoding their validated image;
    /// this entry starts no file or network access of its own.
    pub(crate) fn open_decoded_image(
        &mut self,
        image: std::sync::Arc<gpui::RenderImage>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.areas
            .composer
            .lightbox
            .open(Preview::Local(image.into()), window, cx);
        cx.notify();
    }

    pub(super) fn open_stored_image(
        &mut self,
        file: std::sync::Arc<vessel_app_view::composer::local::LocalFile>,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.open_local_image(ImageSource::from(file.path.clone()), window, cx);
        self.areas.composer.lightbox.local_lease = Some(file);
    }

    pub(super) fn open_local_image(
        &mut self,
        source: ImageSource,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let decoded = match &source {
            ImageSource::Render(image) => Some(std::sync::Arc::clone(image)),
            ImageSource::Image(image) => std::sync::Arc::clone(image).get_render_image(window, cx),
            ImageSource::Resource(_) | ImageSource::Custom(_) => None,
        };
        if let Some(image) = decoded {
            self.open_decoded_image(image, window, cx);
            return;
        }
        self.areas
            .composer
            .lightbox
            .open(Preview::Local(source), window, cx);
        cx.notify();
    }

    /// Open a typed node attachment using the shared bounded read and viewer.
    pub(crate) fn open_sent_image(
        &mut self,
        sent: Sent,
        file: AttachmentRef,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        self.areas
            .composer
            .lightbox
            .open(Preview::Sent { sent, file }, window, cx);
        cx.notify();
    }

    pub(super) fn close_image(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let lightbox = &mut self.areas.composer.lightbox;
        lightbox.tips = vessel_app_kit::Tooltips::new();
        lightbox.preview = None;
        drop(lightbox.local_lease.take());
        lightbox.viewport = None;
        lightbox.gesture = None;
        if let Some(focus) = lightbox.previous.take() {
            focus.focus(window, cx);
        }
        cx.notify();
    }

    fn image_scrolled(&mut self, event: &ScrollEvent, cx: &mut Context<'_, Self>) {
        if let Some(at) = pointer(event.position)
            && let Some(viewport) = &mut self.areas.composer.lightbox.viewport
        {
            let delta = match event.delta {
                ScrollDelta::Pixels(p) => f32::from(p.y),
                ScrollDelta::Lines(p) => p.y.mul_add(vessel_app_kit::theme::spacing::S4, 0.0),
            };
            viewport.scroll(delta, at);
            cx.notify();
        }
        cx.stop_propagation();
    }

    fn image_pinched(&mut self, event: &PinchEvent, cx: &mut Context<'_, Self>) {
        if let Some(at) = pointer(event.position)
            && let Some(viewport) = &mut self.areas.composer.lightbox.viewport
        {
            viewport.zoom(event.delta.mul_add(1.0, 1.0), at);
            cx.notify();
        }
        cx.stop_propagation();
    }

    fn image_moved(&mut self, event: &MouseMoveEvent, cx: &mut Context<'_, Self>) {
        let Some(at) = pointer(event.position) else {
            return;
        };
        let lightbox = &mut self.areas.composer.lightbox;
        let Some(gesture) = lightbox.gesture else {
            return;
        };
        let from = match gesture {
            Gesture::Pressed(from) | Gesture::Dragged(from) => from,
        };
        if at == from {
            return;
        }
        if let Some(viewport) = &mut lightbox.viewport {
            viewport.drag(from, at);
        }
        lightbox.gesture = Some(Gesture::Dragged(at));
        cx.notify();
    }

    fn image_released(
        &mut self,
        event: &MouseUpEvent,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let lightbox = &mut self.areas.composer.lightbox;
        let pressed = matches!(lightbox.gesture.take(), Some(Gesture::Pressed(_)));
        if pressed
            && let Some(at) = pointer(event.position)
            && lightbox
                .viewport
                .is_none_or(|viewport| !viewport.contains(at))
        {
            self.close_image(window, cx);
        }
    }
}

fn pointer(point: Point<Pixels>) -> Option<Position> {
    Position::new(f32::from(point.x), f32::from(point.y))
}

/// Loading and refusal remain distinct so a failed read cannot spin forever.
enum PictureState {
    Pending,
    Failed,
    Ready(ImageSource),
}

fn source(shell: &Shell) -> Option<PictureState> {
    match shell.areas.composer.lightbox.preview.as_ref()? {
        Preview::Local(source) => Some(PictureState::Ready(source.clone())),
        Preview::Sent { sent, file } => {
            let picture = shell.areas.composer.sent.borrow_mut().ask(sent);
            Some(match picture {
                Picture::Wanted => {
                    shell.sent_reads();
                    PictureState::Pending
                }
                Picture::Reading => PictureState::Pending,
                Picture::Failed => PictureState::Failed,
                Picture::Ready(bytes) => PictureState::Ready(
                    shell.areas.composer.pictures.of(&bytes, file.mime.as_str()),
                ),
            })
        }
    }
}

#[cfg(test)]
mod tests;
