//! The effort slider, drawn as the local Codex app's model card draws it
//! (the owner, 2026-11-14: "learn the local Codex app's slider"): a 24-tall
//! round track at 10 % ink, filled in the accent to the thumb's centre, a
//! 4-point dot per stop 13 in from either end, and a white 28-point thumb
//! that grows to 32 while hovered or pressed. At the top stop the fill is
//! a moving purple field revealed from the thumb, with a burst at the
//! thumb; with Fast on, white dots stream back from the thumb.
//!
//! [Decision Log]
//! - Purpose: the owner found the previous slider (a flat white disc on a
//!   grey pill, a jump from stop to stop) low quality.
//! - Constraint: the kit knows no view types; the slider's logic (which
//!   stop a pointer is on, the glide, the spring, the decorations through
//!   time) is in `view::slider`, under test.
//! - Choice: this control draws a [`Paint`] the caller computes each frame
//!   (from `view::slider::state::Motion`) and reports the pointer in
//!   points along the track ([`Pointer`]), the wheel's units and the arrow
//!   keys ([`Events`]). While the paint says something moves it asks for
//!   the display's next frame (GPUI's frame callback, at the display's
//!   rate: the thumb, the fill, Fast's dots, the field and the burst all
//!   advance once per frame drawn); when it rests or is not drawn nothing
//!   ticks. Until 0.0.0.0.14 the decorations ran on a 30-a-second timer,
//!   which read as choppy on the owner's GPU (words 41-42).
//! - The field: the caller gives a sampler (colour and opacity at a point
//!   of the fill); the control samples it every 2 points into one small
//!   image per frame, painted over the fill with its left corners rounded,
//!   and drops the previous frame's image from the atlas.
//! - Differences from the reference, named: the fill is Vessel's accent
//!   (the reference's default `#339cff`); the track's edge is Vessel's
//!   `border` token and the thumb's edge `border_accent` (the reference's
//!   desktop values were not in its bundle); a hovered stop grows but is
//!   not darkened (GPUI has no brightness filter).

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use gpui::prelude::*;
use gpui::{
    AnyElement, App, Bounds, BoxShadow, Corners, ElementId, FocusHandle, Hsla, MouseButton,
    MouseMoveEvent, MouseUpEvent, Pixels, RenderImage, ScrollDelta, ScrollEvent, Window, canvas,
    div, hsla, point, px, white,
};

use crate::theme::Tokens;

/// The track's height.
const TRACK: f32 = 24.0;
/// The control's height with a pointer.
const CONTROL: f32 = 28.0;
/// The touch target's height.
const TOUCH: f32 = 44.0;
/// The block around the control: its height and side padding.
const BLOCK: f32 = 32.0;
const BLOCK_SIDE: f32 = 6.0;
/// The first and last stop's distance from the track's ends.
const INSET: f32 = 13.0;
/// A stop's dot.
const DOT: f32 = 4.0;
/// A stop's hit area.
const DOT_HIT: f32 = 16.0;
/// The thumb at rest.
const THUMB: f32 = 28.0;
/// The thumb's hit area.
const THUMB_HIT: f32 = 34.0;
/// A burst particle.
const BURST_DOT: f32 = 5.0;
/// A fast dot.
const FAST_DOT: f32 = 3.0;
/// The field's sampling step, points.
const FIELD_STEP: f32 = 2.0;
/// A line of wheel scrolling, in the reference's units.
const WHEEL_LINE: f32 = 30.0;

/// A particle: its centre's offset, scale, opacity and colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Particle {
    /// Across.
    pub x: f32,
    /// Down.
    pub y: f32,
    /// Scale.
    pub scale: f32,
    /// Opacity.
    pub opacity: f32,
    /// Colour.
    pub colour: Hsla,
}

/// A stop's dot: its sideways shift, scale, opacity, and whether it is
/// under the fill.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stop {
    /// Sideways shift, points.
    pub shift: f32,
    /// Scale.
    pub scale: f32,
    /// Opacity.
    pub opacity: f32,
    /// Under the fill.
    pub filled: bool,
}

/// The field's colour and opacity (straight, 0 to 1) at a point `x`
/// points from the track's start and `y` up the track (0 bottom, 1 top),
/// with the fill `fill` points wide on a track `track` points wide.
pub type Sampler = Rc<dyn Fn(f32, f32, f32, f32) -> [f32; 4]>;

/// What one frame draws.
#[derive(Clone)]
pub struct Paint {
    /// The thumb's place between the first and last stop (`None`: no stop
    /// in effect, nothing filled and no thumb).
    pub fraction: Option<f32>,
    /// The thumb's scale.
    pub thumb: f32,
    /// The stops.
    pub stops: Vec<Stop>,
    /// The burst, offsets from the thumb's centre.
    pub burst: Vec<Particle>,
    /// Fast's dots: `x` the share of the track's width back from the
    /// thumb, `y` the share of its height from the top.
    pub dots: Vec<Particle>,
    /// The field over the fill, when at the top stop.
    pub field: Option<Sampler>,
    /// The fill's colour.
    pub fill: Hsla,
    /// Whether anything moves (the display's next frame is asked for).
    pub moving: bool,
    /// Whether the slider has the keys and the last input was a key: the
    /// thumb shows the focus ring (the reference's `:focus-visible`).
    pub ring: Option<Hsla>,
}

impl std::fmt::Debug for Paint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Paint")
            .field("fraction", &self.fraction)
            .field("thumb", &self.thumb)
            .field("stops", &self.stops.len())
            .field("field", &self.field.is_some())
            .field("moving", &self.moving)
            .finish_non_exhaustive()
    }
}

/// A callback with a number.
pub type With<T> = Rc<dyn Fn(T, &mut Window, &mut App)>;
/// A callback.
pub type Then = Rc<dyn Fn(&mut Window, &mut App)>;

/// What the control reports.
#[derive(Clone)]
#[expect(missing_debug_implementations, reason = "callbacks only")]
pub struct Events {
    /// A press on the track.
    pub press: With<Pointer>,
    /// The pointer, pressed, anywhere in the window.
    pub drag: With<Pointer>,
    /// The release.
    pub release: Then,
    /// The pointer onto or off the thumb.
    pub hover: With<bool>,
    /// The wheel, in the reference's units (positive: on).
    pub wheel: With<f32>,
    /// An arrow key: -1 back, +1 on.
    pub key: With<i32>,
}

/// A slider's own state, kept by its screen.
#[derive(Debug, Clone)]
pub struct SliderState {
    focus: FocusHandle,
    /// Where the track was drawn last frame.
    track: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// Whether the pointer is pressed on it.
    pressed: Rc<Cell<bool>>,
    /// The field's image last painted, dropped when the next is.
    field: Rc<RefCell<Option<Arc<RenderImage>>>>,
}

impl SliderState {
    /// A slider not pressed.
    pub fn new(cx: &mut App) -> Self {
        Self {
            // A tab stop: the kit's root moves the keys with Tab
            // (`focus_next`), and the slider must be reachable that way.
            focus: cx.focus_handle().tab_stop(true),
            track: Rc::new(Cell::new(None)),
            pressed: Rc::new(Cell::new(false)),
            field: Rc::new(RefCell::new(None)),
        }
    }

    /// Give it the keys (the arrow keys move it).
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.focus.focus(window, cx);
    }

    /// Whether it has the keys and they were the last input (the ring
    /// shows then, not after a click).
    #[must_use]
    pub fn keyboard_focused(&self, window: &Window) -> bool {
        self.focus.is_focused(window) && window.last_input_was_keyboard()
    }
}

/// A pointer over the track, in points: from the track's start (beyond
/// either end while dragging), and the track's width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pointer {
    /// Points from the track's start.
    pub x: f32,
    /// The track's width.
    pub width: f32,
}

/// The pointer at `x` over `track`.
fn pointer(x: Pixels, track: Bounds<Pixels>) -> Pointer {
    Pointer {
        x: f32::from(track.left()).mul_add(-1.0, f32::from(x)),
        width: f32::from(track.size.width),
    }
}

/// Where along a track `width` wide a point `fraction` of the way between
/// the first and last stop is drawn.
fn along(fraction: f32, width: f32) -> f32 {
    fraction.mul_add(INSET.mul_add(-2.0, width).max(0.0), INSET)
}

/// The track's width before it was first measured: the card's (254, less
/// its 4 padding, the block's 2 margins and 6 padding on each side).
const FIRST_WIDTH: f32 = 230.0;

/// A slider drawing `paint`. `touch` makes the target 44 tall.
#[must_use]
pub fn slider(
    id: impl Into<ElementId>,
    state: &SliderState,
    paint: &Paint,
    touch: bool,
    tokens: &Tokens,
    events: &Events,
) -> AnyElement {
    let id = id.into();
    let width = state
        .track
        .get()
        .map_or(FIRST_WIDTH, |track| f32::from(track.size.width));
    let height = if touch { TOUCH } else { CONTROL };
    let thumb_x = paint.fraction.map(|fraction| along(fraction, width));
    let track = track(state, paint, thumb_x, width, tokens);
    let mut root = div()
        .id(id)
        .track_focus(&state.focus)
        .relative()
        .w_full()
        .h(px(height))
        .flex()
        .items_center()
        .child(track);
    if let Some(centre) = thumb_x {
        root = root
            .children(paint.burst.iter().map(|particle| {
                dot_at(
                    centre.mul_add(1.0, particle.x),
                    height.mul_add(0.5, particle.y),
                    BURST_DOT,
                    particle,
                )
            }))
            .child(thumb(centre, height, paint, tokens, events));
    }
    root.child(frames(state, paint, events))
        .on_mouse_down(MouseButton::Left, press(state, events))
        .on_mouse_up(MouseButton::Left, release(state, events))
        .on_mouse_up_out(MouseButton::Left, release(state, events))
        .on_scroll(wheel(events))
        .on_key_down({
            let key = Rc::clone(&events.key);
            move |event, window, cx| {
                let step = match event.keystroke.key.as_str() {
                    "left" => -1,
                    "right" => 1,
                    _ => return,
                };
                cx.stop_propagation();
                key(step, window, cx);
            }
        })
        .into_any_element()
}

/// The block the reference sets the control in: 32 tall, 2 out and 6 in
/// at the sides.
#[must_use]
pub fn block(slider: AnyElement) -> AnyElement {
    div()
        .h(px(BLOCK))
        .mx(px(2.0))
        .px(px(BLOCK_SIDE))
        .flex()
        .flex_col()
        .justify_center()
        .child(slider)
        .into_any_element()
}

/// The track: the 10 % ink pill, the fill, the field, Fast's dots and the
/// stops.
fn track(
    state: &SliderState,
    paint: &Paint,
    thumb_x: Option<f32>,
    width: f32,
    tokens: &Tokens,
) -> AnyElement {
    let measure = Rc::clone(&state.track);
    // No border on the track itself: GPUI places absolute children inside
    // a border, so the fill (24 tall) sat half a point low and showed
    // under the track (the owner, on 0.0.0.0.13, word 26). The hairline
    // is a layer of its own, under the fill, as the reference's inset
    // shadow is under its range.
    let radius = px(TRACK.mul_add(0.5, 0.0));
    let mut track = div()
        .relative()
        .w_full()
        .h(px(TRACK))
        .rounded(radius)
        .bg(tokens.neutral.foreground.opacity(0.10))
        .child(
            div()
                .absolute()
                .size_full()
                .rounded(radius)
                .border(px(0.5))
                .border_color(tokens.neutral.border),
        )
        .child(
            canvas(
                move |bounds, _window, _cx| measure.set(Some(bounds)),
                |_, (), _, _| {},
            )
            .absolute()
            .size_full(),
        );
    if let Some(fill) = thumb_x {
        // The fill is a whole pill whose right cap is centred on the
        // thumb's centre: its left end is always the track's 12-point cap
        // (a fill narrower than the track is tall had its corners cut to
        // half its width, the "square" end at the lowest stop the owner
        // saw), and its right cap, 12 round, lies under the 14-round thumb.
        let span = fill.mul_add(1.0, TRACK.mul_add(0.5, 0.0));
        let mut filled = div()
            .absolute()
            .left_0()
            .top_0()
            .bottom_0()
            .w(px(span))
            .rounded(radius)
            .overflow_hidden();
        filled = match &paint.field {
            // The field's image carries the fill's colour where it is not
            // yet revealed, so nothing is painted under it: a fill under
            // the image showed as a one-pixel rim at fractional scales,
            // where the image's bounds are snapped and the fill's are not.
            Some(sampler) => {
                filled.child(field(state, Rc::clone(sampler), fill, width, paint.fill))
            }
            None => filled.bg(paint.fill),
        };
        filled = filled.children(paint.dots.iter().map(|particle| {
            let x = particle.x.mul_add(width.mul_add(-1.0, 0.0), fill);
            let y = particle.y.mul_add(TRACK, 0.0);
            // GPUI clips to rectangles: a dot near the track's round end
            // fades by how much of it (and its glow) is inside the pill.
            let mut particle = *particle;
            particle.opacity = particle
                .opacity
                .mul_add(inside_pill(x, y, FAST_DOT.mul_add(1.5, 0.0), width), 0.0);
            dot_at(x, y, FAST_DOT, &particle).shadow(vec![BoxShadow {
                color: hsla(0.0, 0.0, 1.0, 0.34),
                offset: point(px(0.0), px(0.0)),
                blur_radius: px(5.0),
                spread_radius: px(0.0),
                inset: false,
            }])
        }));
        track = track.child(filled);
    }
    let stops = paint.stops.len();
    let last = stops.saturating_sub(1);
    track
        .children(paint.stops.iter().enumerate().map(|(index, stop)| {
            let fraction = if last == 0 {
                0.0
            } else {
                f32::from(u16::try_from(index).unwrap_or(0))
                    .mul_add(f32::from(u16::try_from(last).unwrap_or(1)).recip(), 0.0)
            };
            let x = along(fraction, width).mul_add(1.0, stop.shift);
            let colour = if stop.filled {
                white().opacity(0.30)
            } else {
                tokens.neutral.foreground.opacity(0.35)
            };
            let size = DOT.mul_add(stop.scale, 0.0);
            let centre = TRACK.mul_add(0.5, 0.0);
            // A stop sliding away (Fast) leaves through the round end, not
            // past it: faded by how much of it is inside the pill.
            let opacity = stop
                .opacity
                .mul_add(inside_pill(x, centre, size.mul_add(0.5, 0.0), width), 0.0);
            div()
                .absolute()
                .left(px(DOT_HIT.mul_add(-0.5, x)))
                .top(px(DOT_HIT.mul_add(-0.5, centre)))
                .size(px(DOT_HIT))
                .flex()
                .items_center()
                .justify_center()
                .opacity(opacity)
                .group("slider-stop")
                .child(
                    div()
                        .size(px(size))
                        .rounded_full()
                        .bg(colour)
                        .group_hover("slider-stop", move |style| {
                            style.size(px(size.mul_add(2.0, 0.0)))
                        }),
                )
        }))
        .into_any_element()
}

/// How much of a round mark of `radius` centred at (`x`, `y`) lies inside
/// the track's pill, `width` wide and 24 tall: 1 inside, 0 outside, fading
/// over a point at the outline. GPUI's content masks are rectangles, so
/// the round ends are cut here, in what is painted.
#[expect(clippy::float_arithmetic, reason = "a distance to the pill's outline")]
fn inside_pill(x: f32, y: f32, radius: f32, width: f32) -> f32 {
    let half = TRACK * 0.5;
    let centre_x = x.clamp(half, (width - half).max(half));
    let (dx, dy) = (x - centre_x, y - half);
    let distance = (dx * dx + dy * dy).sqrt();
    (half - distance - radius + 1.0).clamp(0.0, 1.0)
}

/// A round particle of `size` centred at (`x`, `y`).
fn dot_at(x: f32, y: f32, size: f32, particle: &Particle) -> gpui::Div {
    let size = size.mul_add(particle.scale, 0.0);
    div()
        .absolute()
        .left(px(size.mul_add(-0.5, x)))
        .top(px(size.mul_add(-0.5, y)))
        .size(px(size))
        .rounded_full()
        .bg(particle.colour)
        .opacity(particle.opacity)
}

/// The thumb, white with a hairline edge and a faint shadow, at `centre`;
/// its 34-point hit area reports hovering.
fn thumb(centre: f32, height: f32, paint: &Paint, tokens: &Tokens, events: &Events) -> AnyElement {
    let size = THUMB.mul_add(paint.thumb, 0.0);
    let ring = paint.ring;
    let hover = Rc::clone(&events.hover);
    div()
        .id("slider-thumb")
        .absolute()
        .left(px(THUMB_HIT.mul_add(-0.5, centre)))
        .top(px(THUMB_HIT.mul_add(-0.5, height.mul_add(0.5, 0.0))))
        .size(px(THUMB_HIT))
        .flex()
        .items_center()
        .justify_center()
        .on_hover(move |over, window, cx| hover(*over, window, cx))
        .child(
            div()
                .size(px(size))
                .flex_none()
                .rounded_full()
                .bg(white())
                .border(px(0.5))
                .border_color(tokens.neutral.border_accent)
                .shadow(
                    // The reference's faint shadow; with the keys, its
                    // 2-point ring around the thumb.
                    std::iter::once(BoxShadow {
                        color: hsla(0.0, 0.0, 0.0, 0.10),
                        offset: point(px(0.0), px(0.0)),
                        blur_radius: px(2.0),
                        spread_radius: px(0.0),
                        inset: false,
                    })
                    .chain(ring.map(|colour| BoxShadow {
                        color: colour,
                        offset: point(px(0.0), px(0.0)),
                        blur_radius: px(0.0),
                        spread_radius: px(2.0),
                        inset: false,
                    }))
                    .collect(),
                ),
        )
        .into_any_element()
}

/// The field over the fill, sampled every 2 points into one image.
fn field(state: &SliderState, sampler: Sampler, fill: f32, width: f32, under: Hsla) -> AnyElement {
    let last = Rc::clone(&state.field);
    canvas(
        |_, _, _| {},
        move |bounds, (), window, cx| {
            let span = f32::from(bounds.size.width);
            let Some(image) = field_image(&sampler, fill, span, width, under) else {
                return;
            };
            if let Some(previous) = last.borrow_mut().replace(Arc::clone(&image)) {
                cx.drop_image(previous, Some(window));
            }
            let radius = px(TRACK.mul_add(0.5, 0.0));
            let corners = Corners::all(radius);
            if let Err(error) = window.paint_image(bounds, bounds, corners, image, 0, false) {
                tracing::warn!(%error, "the slider's field could not be drawn");
            }
        },
    )
    .absolute()
    .left_0()
    .top_0()
    .size_full()
    .into_any_element()
}

/// The field's samples as an image (BGRA, opaque, as GPUI's images are),
/// `span` points wide, the thumb `fill` points along: where the field is
/// not yet revealed the sample is the fill's own colour `under`.
fn field_image(
    sampler: &Sampler,
    fill: f32,
    span: f32,
    width: f32,
    under: Hsla,
) -> Option<Arc<RenderImage>> {
    let base = under.to_rgb();
    let columns = (span.mul_add(FIELD_STEP.recip(), 0.0).ceil()).clamp(1.0, 2048.0);
    let rows = TRACK.mul_add(FIELD_STEP.recip(), 0.0);
    let (count_x, count_y) = (whole(columns), whole(rows));
    let mut bytes = Vec::with_capacity(
        usize::try_from(count_x.saturating_mul(count_y).saturating_mul(4)).unwrap_or(0),
    );
    for row in 0..count_y {
        let y = centre_of(row).mul_add(FIELD_STEP, 0.0);
        let up = y.mul_add(TRACK.recip().mul_add(-1.0, 0.0), 1.0);
        for column in 0..count_x {
            let x = centre_of(column).mul_add(FIELD_STEP, 0.0);
            let [red, green, blue, shown] = sampler(x, up, fill, width);
            let over = |field: f32, fill: f32| {
                field.mul_add(shown, fill.mul_add(shown.mul_add(-1.0, 1.0), 0.0))
            };
            bytes.extend([
                byte(over(blue, base.b)),
                byte(over(green, base.g)),
                byte(over(red, base.r)),
                u8::MAX,
            ]);
        }
    }
    let buffer = image::RgbaImage::from_raw(count_x, count_y, bytes)?;
    Some(Arc::new(RenderImage::new(vec![image::Frame::new(buffer)])))
}

/// Sample `index`'s centre, in steps.
fn centre_of(index: u32) -> f32 {
    f32::from(u16::try_from(index).unwrap_or(u16::MAX)).mul_add(1.0, 0.5)
}

/// A count of samples (1 to 2048) as a whole number.
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped to 1..=2048 and whole before the cast"
)]
fn whole(count: f32) -> u32 {
    count.clamp(1.0, 2048.0).round() as u32
}

/// A channel, 0 to 1, as a byte.
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped to 0..=255 and rounded before the cast"
)]
fn byte(channel: f32) -> u8 {
    channel.mul_add(255.0, 0.0).clamp(0.0, 255.0).round() as u8
}

/// Asks for the next frame while the paint moves, and while pressed hears
/// the pointer's moves anywhere in the window.
fn frames(state: &SliderState, paint: &Paint, events: &Events) -> AnyElement {
    let pressed = Rc::clone(&state.pressed);
    let track = Rc::clone(&state.track);
    let drag = Rc::clone(&events.drag);
    let moving = paint.moving;
    canvas(
        |_, _, _| {},
        move |_bounds, (), window, _cx| {
            // Asked only while the paint moves (under held motion the
            // caller's frames rest): a resting or closed card does not
            // tick. Each frame drawn while it moves asks for the next.
            if moving {
                crate::frame_log::note("slider");
                window.request_animation_frame();
            } else {
                crate::frame_log::note("still");
            }
            if !pressed.get() {
                return;
            }
            let pressed = Rc::clone(&pressed);
            let track = Rc::clone(&track);
            let drag = Rc::clone(&drag);
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                if phase.bubble()
                    && pressed.get()
                    && let Some(bounds) = track.get()
                {
                    drag(pointer(event.position.x, bounds), window, cx);
                }
            });
        },
    )
    .absolute()
    .size_full()
    .into_any_element()
}

fn press(
    state: &SliderState,
    events: &Events,
) -> impl Fn(&gpui::MouseDownEvent, &mut Window, &mut App) + 'static {
    let pressed = Rc::clone(&state.pressed);
    let track = Rc::clone(&state.track);
    let focus = state.focus.clone();
    let press = Rc::clone(&events.press);
    move |event, window, cx| {
        let Some(bounds) = track.get() else {
            return;
        };
        pressed.set(true);
        focus.focus(window, cx);
        press(pointer(event.position.x, bounds), window, cx);
    }
}

fn release(
    state: &SliderState,
    events: &Events,
) -> impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static {
    let pressed = Rc::clone(&state.pressed);
    let release = Rc::clone(&events.release);
    move |_event, window, cx| {
        if pressed.replace(false) {
            release(window, cx);
        }
    }
}

fn wheel(events: &Events) -> impl Fn(&ScrollEvent, &mut Window, &mut App) + 'static {
    let wheel = Rc::clone(&events.wheel);
    move |event, window, cx| {
        if event.modifiers.control {
            return;
        }
        // The reference takes the larger of the two axes, up and right
        // moving on.
        let (across, up) = match event.delta {
            ScrollDelta::Pixels(delta) => {
                (f32::from(delta.x).mul_add(-1.0, 0.0), f32::from(delta.y))
            }
            ScrollDelta::Lines(delta) => (
                delta.x.mul_add(WHEEL_LINE.mul_add(-1.0, 0.0), 0.0),
                delta.y.mul_add(WHEEL_LINE, 0.0),
            ),
        };
        let units = if across.abs() > up.abs() { across } else { up };
        if units.abs() > 0.0 {
            cx.stop_propagation();
            wheel(units, window, cx);
        }
    }
}

/// Paseo's activity indicator: three quarters of a ring turning, a turn
/// every 0.8 s, in `colour`, `size` across. Frame-driven while drawn, as
/// the slider's motion.
#[must_use]
pub fn spinner(id: impl Into<ElementId>, size: f32, colour: Hsla) -> AnyElement {
    let vector = gpui::svg()
        .path(crate::assets::Glyph::Spinner.path())
        .size(px(size))
        .flex_shrink_0()
        .text_color(colour);
    crate::loading::rotate(id, vector, crate::loading::LEGACY_PERIOD)
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, point, px, size};

    use super::{along, pointer};

    #[test]
    fn a_pointer_is_measured_from_the_track_start() {
        let track = Bounds::new(point(px(100.0), px(0.0)), size(px(200.0), px(24.0)));
        assert!((pointer(px(90.0), track).x + 10.0).abs() < 1e-6);
        assert!((pointer(px(150.0), track).x - 50.0).abs() < 1e-6);
        assert!((pointer(px(150.0), track).width - 200.0).abs() < 1e-6);
    }

    #[test]
    fn a_mark_past_the_round_end_is_not_drawn() {
        // In the middle, inside; at the very left, a mark centred on the
        // end's curve is outside; one sliding 85 to the left is gone.
        assert!((super::inside_pill(100.0, 12.0, 2.0, 230.0) - 1.0).abs() < 1e-6);
        assert!(super::inside_pill(1.0, 12.0, 2.0, 230.0) < 1e-6);
        assert!(super::inside_pill(-60.0, 12.0, 2.0, 230.0) < 1e-6);
        // A fast dot by the top edge near the left end is cut.
        assert!(super::inside_pill(4.0, 3.0, 2.0, 230.0) < 1e-6);
        assert!((super::inside_pill(13.0, 12.0, 2.0, 230.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn stops_sit_thirteen_in_from_either_end() {
        assert!((along(0.0, 230.0) - 13.0).abs() < 1e-4);
        assert!((along(1.0, 230.0) - 217.0).abs() < 1e-4);
    }
}
