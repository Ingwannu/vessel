//! The Files tab's panels drawn (`panels.rs`): Paseo's Preview|Source
//! control in the file's head; the Markdown preview through the kit's
//! Markdown, with relative links opening files here; and the image panel,
//! fitted to the panel and zoomable as the composer's lightbox is (wheel
//! at the cursor, pinch, drag once zoomed in, a double click fits again),
//! with Zoom in, Zoom out and Reset view in its top right corner while
//! the pointer is over it.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    MouseMoveEvent, ObjectFit, PinchEvent, ScrollDelta, ScrollEvent, StyledImage as _, img,
};
use vessel_app_kit::assets::Glyph;
use vessel_app_kit::chat_markdown::{Options, actions::Actions, chat_markdown_with};
use vessel_app_kit::icon_chrome::Presence;
use vessel_app_kit::segmented::{SegmentSize, segment, segmented};
use vessel_app_kit::theme::{radius, spacing};
use vessel_app_kit::toolbar_button::{ToolbarButton, ToolbarConfig};
use vessel_app_kit::ui::{
    AnyElement, Bounds, Context, FluentBuilder as _, InteractiveElement as _, IntoElement as _,
    MouseButton, ParentElement as _, Pixels, StatefulInteractiveElement as _, Styled as _, Window,
    canvas, div, px, relative,
};
use vessel_app_kit::{ButtonState, ControlSizing, Tokens, Type, TypeExt as _};
use vessel_app_view::composer::viewer::{
    Extent, Fit, MAX_SCALE, MIN_SCALE, Position, Viewport, ZOOM_IN, ZOOM_OUT,
};
use vessel_app_view::workspace::front_matter::{FrontMatterRow, markdown_document};
use vessel_app_view::workspace::markdown_paths::{self, Target};
use vessel_app_view::workspace::viewers::{Side, preview_fits};

use super::frame::note;
use super::panels::{ImagePanel, ImageState};
use super::{Shown, Viewer};
use crate::shell::Shell;

/// The head's Preview|Source control, for a file with a Preview once its
/// text is read.
pub(super) fn side_control(
    shell: &Shell,
    viewer: &Viewer,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    if !viewer.look.panel.has_preview() || !matches!(viewer.text, Shown::Text { .. }) {
        return None;
    }
    let words = shell.t.lang.viewers();
    let chosen = viewer.look.side;
    let one = |id: &'static str, side: Side, label: &'static str, cx: &mut Context<'_, Shell>| {
        segment(id, label, chosen == side, true, SegmentSize::Small, tokens)
            .role(gpui::Role::RadioButton)
            .aria_label(label)
            .on_click(cx.listener(move |shell, _event, _window, cx| shell.code_side(side, cx)))
    };
    Some(
        segmented(SegmentSize::Small)
            .id("code-side")
            .role(gpui::Role::RadioGroup)
            .aria_label(words.file_view)
            .child(one("code-side-preview", Side::Preview, words.preview, cx))
            .child(one("code-side-source", Side::Source, words.source, cx))
            .into_any_element(),
    )
}

/// The Markdown preview of the text shown (the edit's, while one is open),
/// or Paseo's words for a file past its display budget.
pub(super) fn markdown(
    shell: &Shell,
    viewer: &Viewer,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.t.lang.viewers();
    let text = match (&viewer.edit, &viewer.text) {
        (Some(editing), _) if editing.model.version().is_some() => {
            editing.model.content().to_owned()
        }
        // Read only in part (a phone's width reads a page at a time): the
        // Preview draws the whole document or says it cannot.
        (_, Shown::Text { more: true, .. }) => {
            return note(words.too_large, tokens).into_any_element();
        }
        (_, Shown::Text { lines, .. }) => lines.join("\n"),
        _ => return note(shell.words().labels().loading, tokens).into_any_element(),
    };
    if u64::try_from(text.len()).is_ok_and(|size| !preview_fits(size)) {
        return note(words.too_large, tokens).into_any_element();
    }
    let from = viewer.path.clone();
    let folder = shell
        .areas
        .workspace
        .code
        .cwd()
        .map(|cwd| cwd.to_string_lossy().into_owned());
    let shell_handle = cx.entity().downgrade();
    let actions = Actions::new(move |href, _window, cx| {
        match markdown_paths::target(href.as_str(), &from, folder.as_deref()) {
            Target::Web(url) => cx.open_url(&url),
            Target::File(path) => {
                let _gone = shell_handle.update(cx, |shell, cx| shell.open_file(path, cx));
            }
            Target::Data(_) | Target::None => {}
        }
    })
    .with_images(viewer.look.pictures.resolver());
    let document = markdown_document(&text);
    let version = text_version(&document.body);
    let body = chat_markdown_with(
        "code-markdown-preview",
        &document.body,
        version,
        Options {
            actions: Some(actions),
            ..Options::default()
        },
        tokens,
        window,
        cx,
    );
    cx.defer_in(window, Shell::preview_pictures);
    div()
        .id("code-markdown-scroll")
        .size_full()
        .overflow_y_scroll()
        .child(
            div()
                .max_w(px(MARKDOWN_COLUMN))
                .mx_auto()
                .px(px(spacing::S6))
                .py(px(spacing::S4))
                .children(front_matter(&document.front_matter, tokens))
                .child(body),
        )
        .into_any_element()
}

/// Paseo's front matter table (`markdown-preview/index.tsx`): a bordered
/// box, radius 8, 24 above the body; each row a key cell (30% of the
/// width, at most 180, on the second surface, the key mono in the muted
/// ink) and a value cell, both 12 across and 8 down, 14/20 text alike; a
/// hairline between rows.
fn front_matter(rows: &[FrontMatterRow], tokens: &Tokens) -> Option<AnyElement> {
    if rows.is_empty() {
        return None;
    }
    let neutral = tokens.neutral;
    let table = div()
        .flex()
        .flex_col()
        .mb(px(spacing::S6))
        .border_1()
        .border_color(neutral.border)
        .rounded(px(radius::R2))
        .overflow_hidden()
        .children(rows.iter().enumerate().map(|(index, row)| {
            div()
                .flex()
                .flex_row()
                .items_stretch()
                .when(index > 0, |line| line.border_t_1().border_color(neutral.border))
                .child(
                    div()
                        .flex_basis(relative(KEY_SHARE))
                        .max_w(px(KEY_MAX))
                        .flex_shrink_0()
                        .px(px(spacing::S3))
                        .py(px(spacing::S2))
                        .bg(neutral.surface2)
                        .type_(Type::MonoBody)
                        .text_color(neutral.foreground_muted)
                        .child(row.key.clone()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(spacing::S0))
                        .px(px(spacing::S3))
                        .py(px(spacing::S2))
                        .type_(Type::Label)
                        .font_weight(gpui::FontWeight::NORMAL)
                        .text_color(neutral.foreground)
                        .child(row.value.clone()),
                )
        }));
    Some(table.into_any_element())
}

/// The key cell's share of the row (Paseo's `flexBasis: "30%"`).
const KEY_SHARE: f32 = 0.3;
/// The key cell's widest (Paseo's `maxWidth: 180`).
const KEY_MAX: f32 = 180.0;

/// The reading column's width (the page's 78ch at its 14px body).
const MARKDOWN_COLUMN: f32 = 760.0;

/// A cheap version of `text` for the renderer's re-parse.
fn text_version(text: &str) -> u64 {
    use std::hash::{BuildHasher as _, BuildHasherDefault, DefaultHasher};
    BuildHasherDefault::<DefaultHasher>::default().hash_one(text)
}

/// The image panel: Loading, the error's words, or the image zoomable.
pub(super) fn image(
    shell: &Shell,
    image: &ImagePanel,
    tokens: &Tokens,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.t.lang.viewers();
    let picture = match &image.state {
        ImageState::Loading => return note(words.loading_file, tokens).into_any_element(),
        ImageState::Failed(why) => {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .p(px(spacing::S4))
                .text_color(tokens.danger)
                .child(why.clone())
                .into_any_element();
        }
        ImageState::Ready(picture) => picture,
    };
    let probe = Rc::clone(&image.canvas);
    let measured = probe.get();
    let drawn = measured.and_then(|bounds| {
        let canvas = Extent::new(f32::from(bounds.size.width), f32::from(bounds.size.height))?;
        let decoded = crate::areas::composer::decode_image(
            gpui::ImageSource::Image(std::sync::Arc::clone(picture)),
            window,
            cx,
        )?
        .ok()?;
        let pixels = decoded.size(0);
        let content = Extent::new(
            f32::from(Pixels::from(u32::from(pixels.width))),
            f32::from(Pixels::from(u32::from(pixels.height))),
        )?;
        Some((content, canvas, decoded))
    });
    let mut element = div()
        .id("code-image")
        .size_full()
        .relative()
        .overflow_hidden()
        .role(gpui::Role::Image)
        .child(
            canvas(
                move |bounds, window, _cx| {
                    if probe.get() != Some(bounds) {
                        probe.set(Some(bounds));
                        window.refresh();
                    }
                },
                |_, (), _, _| {},
            )
            .absolute()
            .size_full(),
        );
    if let Some((content, canvas_size, decoded)) = drawn {
        let viewport = image
            .viewport
            .filter(|viewport| viewport.canvas() == canvas_size)
            .unwrap_or_else(|| Viewport::fitted(content, canvas_size, Fit::Panel));
        element = element.child(
            img(decoded)
                .absolute()
                .left(px(viewport.left()))
                .top(px(viewport.top()))
                .w(px(viewport.size().width()))
                .h(px(viewport.size().height()))
                .object_fit(ObjectFit::Contain),
        );
        cx.defer_in(window, move |shell, _window, _cx| {
            shell.image_fitted(viewport)
        });
    }
    element
        .on_hover(cx.listener(|shell, hovered: &bool, _window, cx| {
            if let Some(image) = shell.shown_image() {
                image.hovered = *hovered;
                cx.notify();
            }
        }))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|shell, event: &gpui::MouseDownEvent, _window, cx| {
                if event.click_count >= 2 {
                    shell.image_zoom(Zoom::Reset, cx);
                } else if let Some(image) = shell.shown_image() {
                    image.dragged = at(&image.canvas, event.position);
                }
            }),
        )
        .on_mouse_move(cx.listener(|shell, event: &MouseMoveEvent, _window, cx| {
            shell.image_dragged(event, cx);
        }))
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|shell, _event, _window, _cx| {
                if let Some(image) = shell.shown_image() {
                    image.dragged = None;
                }
            }),
        )
        .on_scroll(cx.listener(|shell, event: &ScrollEvent, _window, cx| {
            shell.image_scrolled_here(event, cx);
        }))
        .on_pinch(cx.listener(|shell, event: &PinchEvent, _window, cx| {
            shell.image_pinched_here(event, cx);
        }))
        .child(toolbar(shell, image, tokens, window, cx))
        .into_any_element()
}

/// A window point as a point of the canvas measured in `probe`.
fn at(probe: &Rc<Cell<Option<Bounds<Pixels>>>>, point: gpui::Point<Pixels>) -> Option<Position> {
    let bounds = probe.get()?;
    Position::new(
        f32::from(point.x - bounds.origin.x),
        f32::from(point.y - bounds.origin.y),
    )
}

/// A toolbar action.
#[derive(Debug, Clone, Copy)]
pub(super) enum Zoom {
    In,
    Out,
    Reset,
}

/// Paseo's viewport toolbar (`zoomable-viewport/toolbar.tsx`), as the
/// lightbox draws it, without its Close.
fn toolbar(
    shell: &Shell,
    image: &ImagePanel,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.t.lang.viewers();
    let sizing = ControlSizing::from_compact(vessel_app_kit::breakpoint::phone(window));
    let presence = if image.hovered || sizing == ControlSizing::Touch {
        Presence::Shown
    } else {
        Presence::Hidden
    };
    let scale = image.viewport.map(Viewport::scale);
    let actions = [
        (
            "code-image-zoom-in",
            Glyph::ZoomIn,
            Zoom::In,
            words.zoom_in,
            scale.is_some_and(|v| v < MAX_SCALE),
        ),
        (
            "code-image-zoom-out",
            Glyph::ZoomOut,
            Zoom::Out,
            words.zoom_out,
            scale.is_some_and(|v| v > MIN_SCALE),
        ),
        (
            "code-image-reset",
            Glyph::Scan,
            Zoom::Reset,
            words.reset_view,
            scale.is_some_and(|v| v != MIN_SCALE),
        ),
    ];
    div()
        .absolute()
        .top(px(spacing::S2))
        .right(px(spacing::S2))
        .flex()
        .gap(px(spacing::S1))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .children(
            actions
                .into_iter()
                .map(|(id, glyph, zoom, label, enabled)| {
                    let availability = if enabled {
                        ButtonState::Enabled
                    } else {
                        ButtonState::Disabled
                    };
                    ToolbarButton::viewport(
                        id,
                        label,
                        glyph,
                        ToolbarConfig::new(sizing, availability),
                        presence,
                        tokens,
                    )
                    .on_click(cx.listener(move |shell, _, _window, cx| shell.image_zoom(zoom, cx)))
                }),
        )
        .into_any_element()
}

impl Shell {
    /// The image panel of the file shown, if it has one.
    fn shown_image(&mut self) -> Option<&mut ImagePanel> {
        self.areas
            .workspace
            .code
            .files
            .viewer
            .as_mut()?
            .look
            .image
            .as_mut()
    }

    /// The viewport drawn this frame, kept for the next input.
    fn image_fitted(&mut self, viewport: Viewport) {
        if let Some(image) = self.shown_image()
            && image.viewport != Some(viewport)
        {
            image.viewport = Some(viewport);
        }
    }

    fn image_zoom(&mut self, zoom: Zoom, cx: &mut Context<'_, Self>) {
        if let Some(viewport) = self.shown_image().and_then(|image| image.viewport.as_mut()) {
            match zoom {
                Zoom::In => viewport.zoom_center(ZOOM_IN),
                Zoom::Out => viewport.zoom_center(ZOOM_OUT),
                Zoom::Reset => viewport.reset(),
            }
            cx.notify();
        }
    }

    fn image_dragged(&mut self, event: &MouseMoveEvent, cx: &mut Context<'_, Self>) {
        let Some(image) = self.shown_image() else {
            return;
        };
        let (Some(from), Some(to)) = (image.dragged, at(&image.canvas, event.position)) else {
            return;
        };
        if let Some(viewport) = image.viewport.as_mut() {
            viewport.drag(from, to);
            image.dragged = Some(to);
            cx.notify();
        }
    }

    fn image_scrolled_here(&mut self, event: &ScrollEvent, cx: &mut Context<'_, Self>) {
        let Some(image) = self.shown_image() else {
            return;
        };
        let focal = at(&image.canvas, event.position);
        if let (Some(focal), Some(viewport)) = (focal, image.viewport.as_mut()) {
            let delta = match event.delta {
                ScrollDelta::Pixels(moved) => f32::from(moved.y),
                ScrollDelta::Lines(moved) => moved.y.mul_add(spacing::S4, 0.0),
            };
            viewport.scroll(delta, focal);
            cx.notify();
        }
        cx.stop_propagation();
    }

    fn image_pinched_here(&mut self, event: &PinchEvent, cx: &mut Context<'_, Self>) {
        let Some(image) = self.shown_image() else {
            return;
        };
        let focal = at(&image.canvas, event.position);
        if let (Some(focal), Some(viewport)) = (focal, image.viewport.as_mut()) {
            viewport.zoom(event.delta.mul_add(1.0, 1.0), focal);
            cx.notify();
        }
        cx.stop_propagation();
    }
}
