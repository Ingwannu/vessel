//! The model-and-effort control and its card, as the local Codex app's
//! model card has them (the owner, 2026-11-14, words 20-24: learn the
//! Codex app's slider; the effort's name opens the detail settings too;
//! its colour change at the top effort and with Fast). One control in the
//! composer's row reads the model, then the effort's word, led by a bolt
//! when Fast is on; pressing it opens the card: a title row (the bolt at
//! the left, the effort's word over the model's name and a chevron as one
//! button opening the model's list, the reset at the right) and under it
//! the slider (`kit::slider`, its logic in `view::slider`).
//!
//! [Decision Log]
//! - Purpose: the owner found the slider low quality and asked for the
//!   effort's word to open the detail list as the model's name does.
//! - Reference values (the Codex app's stylesheet): the card's panel 254
//!   wide with 4 inside; the title row at least 36 tall, 32 in at the
//!   sides; the bolt and the reset 32 squares at its corners with a 26
//!   hover square (round 7); the title one button (4 inside, round 8) whose
//!   content (2 by 4 inside, round 6) takes the hover, the effort's word
//!   over the model's name (small, quieter) and a chevron; while the slider
//!   is dragged onto the top stop the warning covers the row (purple, a
//!   light band crossing it once) and the row's controls are hidden.
//! - Colours: the top stop (`Effort::maximum`) is the reference's purple
//!   (`#ad7bf9` dark, `#924ff7` light) for the word and a bolt that is on;
//!   the slider's fill and an "on" bolt below the top are Vessel's accent.
//! - Differences, named: the word below the top stop is the ink, not the
//!   accent (Vessel's teal accent on the dark card is too faint for text),
//!   and an "on" bolt on the dark card is the teal mixed halfway toward
//!   the ink ([`accent_mark`]);
//!   the card keeps Vessel's popup surface, edge and corners (the
//!   reference's menu surface was not read); the bolt's tooltip has no
//!   dimmed second line (the kit's tooltip is one style).

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use vessel_app_kit::assets::Glyph;
use vessel_app_kit::slider::{self, Events, Paint, Particle, Sampler, SliderState, Stop};
use vessel_app_kit::ui::{
    AnyElement, App, Context, Div, FluentBuilder as _, Hsla, InteractiveElement as _,
    IntoElement as _, ParentElement as _, StatefulInteractiveElement as _, Styled as _, Window,
    div, point, px, rgb,
};
use vessel_app_kit::{PillParts, Scheme, Tokens, Type, TypeExt as _, glyph, popup};
use vessel_app_view::Controls;
use vessel_app_view::effort::{Effort, Step};
use vessel_app_view::slider::flair;
use vessel_app_view::slider::state::{Frame, Input, Motion};

use super::Shown;
use super::pickers::Picker;
use crate::shell::Shell;

/// The card's width.
const WIDTH: f32 = 254.0;
/// Its padding.
const PAD: f32 = 4.0;
/// Its corners (Vessel's popup).
const ROUND: f32 = 14.0;
/// The title row's least height, and its side room for the buttons.
const TITLE: f32 = 36.0;
const TITLE_SIDE: f32 = 32.0;
/// A corner button and its hover square.
const SQUARE: f32 = 32.0;
const SQUARE_HOVER: f32 = 26.0;
/// The bolt's size.
const BOLT: f32 = 20.0;
/// The card's distance above the control.
const ABOVE: f32 = -6.0;

/// The top stop's purple, dark and light.
const PURPLE_DARK: u32 = 0x00ad_7bf9;
const PURPLE_LIGHT: u32 = 0x0092_4ff7;

/// The card's state, kept by the draft.
pub(crate) struct CardState {
    /// Whether it is open.
    pub(crate) open: bool,
    /// Its slider's state.
    pub(crate) slider: SliderState,
    /// The slider through time.
    motion: RefCell<Motion>,
    /// The clock the motion runs on.
    clock: Instant,
    /// Where the bolt is drawn, for its tooltip.
    bolt: vessel_app_kit::tooltip::Place,
}

impl CardState {
    pub(crate) fn new(cx: &mut App) -> Self {
        Self {
            open: false,
            slider: SliderState::new(cx),
            motion: RefCell::new(Motion::default()),
            clock: Instant::now(),
            bolt: Rc::new(std::cell::Cell::new(None)),
        }
    }

    /// Seconds on the card's clock.
    fn now(&self) -> f32 {
        self.clock.elapsed().as_secs_f32()
    }

    /// Open or close it; closing drops the slider's motion.
    pub(crate) fn set_open(&mut self, open: bool) {
        self.open = open;
        if !open {
            self.motion.borrow_mut().close();
        }
    }
}

/// The top stop's purple.
fn purple(tokens: &Tokens) -> Hsla {
    rgb(match tokens.scheme {
        Scheme::Dark => PURPLE_DARK,
        Scheme::Light => PURPLE_LIGHT,
    })
    .into()
}

/// The accent for a small mark (the bolt that is on): Vessel's teal on
/// the light card; on the dark card the teal is 2.0:1 against it
/// (measured, `surface2`), so it is mixed halfway toward the ink there
/// (about 6:1). The slider's fill, a large area, keeps the teal.
fn accent_mark(tokens: &Tokens) -> Hsla {
    match tokens.scheme {
        Scheme::Light => tokens.neutral.accent,
        Scheme::Dark => tokens
            .neutral
            .accent
            .blend(tokens.neutral.foreground.opacity(0.5)),
    }
}

/// The step's word, as the agent says it.
fn word(effort: &Effort, index: Option<usize>) -> Option<&str> {
    index
        .and_then(|index| effort.steps.get(index))
        .map(|step| match step {
            Step::Level { name, .. } | Step::Top { name } => name.as_str(),
        })
}

/// The word's colour: the purple on the top stop, else the ink.
fn word_colour(effort: &Effort, shown: Option<usize>, tokens: &Tokens) -> Hsla {
    if shown.is_some() && shown == effort.maximum() {
        purple(tokens)
    } else {
        tokens.neutral.foreground
    }
}

/// The model control's words: the model, the agent's name when shown, the
/// effort's word and its colour.
pub(crate) struct PillWords {
    pub(crate) model: String,
    pub(crate) agent: Option<String>,
    pub(crate) effort: Option<(String, Option<Hsla>)>,
    pub(crate) fast: bool,
}

/// What the control reads for `controls` (`model` and `agent` from the
/// model's list's words).
pub(crate) fn pill_words(
    controls: Option<&Controls>,
    model: String,
    agent: Option<String>,
    tokens: &Tokens,
) -> PillWords {
    let effort = controls.and_then(|controls| controls.slider.as_ref());
    let fast = effort
        .and_then(|effort| effort.fast.as_ref())
        .is_some_and(vessel_app_view::effort::Switch::on);
    let effort_word = effort.and_then(|effort| {
        let word = word(effort, effort.current)?;
        let colour = (effort.current.is_some() && effort.current == effort.maximum())
            .then(|| purple(tokens));
        Some((word.to_owned(), colour))
    });
    PillWords {
        model,
        agent,
        effort: effort_word,
        fast,
    }
}

/// The control's parts: the bolt when fast mode is on (else Paseo's model
/// glyph), the model, the agent quieter, the effort's word.
pub(crate) fn pill_parts(
    words: &PillWords,
    caret: bool,
    show_agent: bool,
    show_effort: bool,
) -> PillParts {
    PillParts {
        leading: Some(if words.fast {
            Glyph::LightningFilled
        } else {
            Glyph::Bot
        }),
        label: Some(words.model.clone().into()),
        quiet: words.agent.clone().filter(|_| show_agent).map(Into::into),
        after: words
            .effort
            .clone()
            .filter(|_| show_effort)
            .map(|(word, colour)| (word.into(), colour)),
        caret,
        ..PillParts::default()
    }
}

impl Shell {
    /// Press the model control: its card when the agent has levels, else
    /// the model's list.
    pub(crate) fn press_model_control(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let has_levels = matches!(
            self.shown(),
            Some(Shown::Ready(controls)) if controls.slider.is_some()
        );
        if !has_levels {
            self.open_picker(Picker::Model, window, cx);
            return;
        }
        for which in super::pickers::ALL {
            super::pickers::list_of(self, which).close();
        }
        let card = &mut self.areas.draft.effort;
        card.set_open(!card.open);
        if card.open {
            card.slider.focus(window, cx);
            // The card shows the agent's kept lists; opening it asks the
            // node for fresh ones, as every other option control does.
            self.probe(true);
        } else {
            self.focus_controls_composer(window, cx);
        }
        cx.notify();
    }

    /// Close the card.
    pub(crate) fn close_effort_card(&mut self, cx: &mut Context<'_, Self>) {
        if self.areas.draft.effort.open {
            self.areas.draft.effort.set_open(false);
            cx.notify();
        }
    }

    /// Apply `choices` from the card to the draft.
    fn effort_choices(&mut self, pick: impl Fn(&Effort) -> Vec<vessel_proto::OptionChoice>) {
        let Some(Shown::Ready(controls)) = self.shown() else {
            return;
        };
        let Some(effort) = controls.slider.as_ref() else {
            return;
        };
        for choice in pick(effort) {
            self.choose_option(choice);
        }
    }
}

/// The effort control's state on the draft now (a copy), when it has one.
fn effort_now(shell: &Shell) -> Option<Effort> {
    match shell.shown() {
        Some(Shown::Ready(controls)) => controls.slider.clone(),
        _ => None,
    }
}

impl Shell {
    /// The slider pressed at `pointer`.
    fn effort_press(&mut self, pointer: slider::Pointer) {
        let Some(effort) = effort_now(self) else {
            return;
        };
        self.areas.draft.effort.motion.borrow_mut().press(
            along_track(pointer),
            effort.steps.len(),
            effort.current,
        );
    }

    /// The slider dragged to `pointer`: the thumb follows it.
    fn effort_drag(&mut self, pointer: slider::Pointer) {
        let Some(effort) = effort_now(self) else {
            return;
        };
        self.areas
            .draft
            .effort
            .motion
            .borrow_mut()
            .drag(along_track(pointer), effort.steps.len());
    }

    /// The slider released: its stop is chosen when it changed.
    fn effort_release(&mut self) {
        let released = self.areas.draft.effort.motion.borrow_mut().release();
        if let Some(stop) = released {
            self.effort_choices(|effort| effort.choose(stop));
        }
    }

    /// Move the effort `by` stops (a key or the wheel), chosen at once.
    fn effort_step(&mut self, by: i32) {
        let Some(effort) = effort_now(self) else {
            return;
        };
        let last = effort.steps.len().saturating_sub(1);
        let from = effort.current.unwrap_or(0);
        let magnitude = usize::try_from(by.unsigned_abs()).unwrap_or(usize::MAX);
        let to = if by < 0 {
            from.saturating_sub(magnitude)
        } else {
            from.saturating_add(magnitude).min(last)
        };
        if by != 0 && Some(to) != effort.current {
            self.effort_choices(|effort| effort.choose(to));
        }
    }
}

/// The kit's pointer as the slider's model reads it.
const fn along_track(pointer: slider::Pointer) -> vessel_app_view::slider::drag::Pointer {
    vessel_app_view::slider::drag::Pointer {
        x: pointer.x,
        width: pointer.width,
    }
}

/// A slider callback that applies `change` to the shell and redraws.
fn on<T: 'static>(
    cx: &mut Context<'_, Shell>,
    change: impl Fn(&mut Shell, T) + 'static,
) -> slider::With<T> {
    let entity = cx.entity().downgrade();
    Rc::new(move |value, _window, cx| {
        let _updated = entity.update(cx, |shell, cx| {
            change(shell, value);
            cx.notify();
        });
    })
}

/// The slider's callbacks.
fn events(cx: &mut Context<'_, Shell>) -> Events {
    let release = on(cx, |shell, ()| shell.effort_release());
    Events {
        press: on(cx, Shell::effort_press),
        drag: on(cx, Shell::effort_drag),
        release: Rc::new(move |window, cx| release((), window, cx)),
        hover: on(cx, |shell, over| {
            shell.areas.draft.effort.motion.borrow_mut().hover(over);
        }),
        wheel: on(cx, |shell, units| {
            let now = shell.areas.draft.effort.now();
            let steps = shell
                .areas
                .draft
                .effort
                .motion
                .borrow_mut()
                .wheel(units, now);
            shell.effort_step(steps);
        }),
        key: on(cx, Shell::effort_step),
    }
}

/// The card, when open, above the model control.
pub(crate) fn card(
    shell: &Shell,
    controls: Option<&Controls>,
    touch: bool,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let state = &shell.areas.draft.effort;
    if !state.open {
        return None;
    }
    let controls = controls?;
    let effort = controls.slider.as_ref()?;
    let place = shell.areas.draft.features.places[0].get()?;
    let neutral = tokens.neutral;
    let fast_on = effort
        .fast
        .as_ref()
        .is_some_and(vessel_app_view::effort::Switch::on);
    let frame = frame_now(state, effort, fast_on, cx);
    let shown = state.motion.borrow().shown(effort.current);
    let surface = match tokens.scheme {
        Scheme::Light => neutral.surface0,
        Scheme::Dark => neutral.surface2,
    };
    let model_name = controls.model.as_ref().map_or_else(
        || shell.t.lang.options().default_model.to_owned(),
        |model| model.current_name().to_owned(),
    );
    let title = Title {
        effort,
        shown,
        model_name: &model_name,
        warning: frame.warning,
        at_maximum: frame.at_maximum,
        touch,
    };
    let top = title_row(shell, &title, tokens, cx);
    let mut paint = paint(&frame, tokens);
    // The ring when the slider has the keys (Tab reaches it, an arrow key
    // moves it), as the reference's thumb under `:focus-visible`; Vessel's
    // ring token, not the reference's blue.
    paint.ring = state.slider.keyboard_focused(window).then_some(tokens.ring);
    let slider = slider::block(slider::slider(
        "effort-slider",
        &state.slider,
        &paint,
        touch,
        tokens,
        &events(cx),
    ));
    let pill_place = place;
    // Its own layer: the composer under it is not hovered and does not
    // show its text cursor through it (the owner, word 27).
    let card = popup::layer(div().id("effort-card"))
        .w(px(WIDTH))
        .p(px(PAD))
        .flex()
        .flex_col()
        .rounded(px(ROUND))
        .bg(surface)
        .border_1()
        .border_color(neutral.border_accent)
        .shadow(popup::shadow_md(tokens))
        .child(top)
        .child(div().min_h(px(TITLE)).py(px(4.0)).child(slider))
        .on_hover(cx.listener(|shell, over: &bool, _window, cx| {
            let card = &shell.areas.draft.effort;
            let now = card.now();
            card.motion.borrow_mut().engage(*over, now);
            cx.notify();
        }))
        .on_mouse_down_out(
            cx.listener(move |shell, event: &gpui::MouseDownEvent, _window, cx| {
                // A press on the control itself toggles the card there.
                if !pill_place.contains(&event.position) {
                    shell.close_effort_card(cx);
                }
            }),
        )
        .on_key_down(
            cx.listener(|shell, event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    shell.areas.draft.effort.set_open(false);
                    shell.focus_controls_composer(window, cx);
                    cx.notify();
                }
            }),
        );
    Some(
        gpui::deferred(
            gpui::anchored()
                .position(point(place.left(), place.top()))
                .anchor(gpui::Anchor::BottomLeft)
                .offset(point(px(0.0), px(ABOVE)))
                .snap_to_window_with_margin(px(8.0))
                .child(card),
        )
        .with_priority(3)
        .into_any_element(),
    )
}

/// The slider's frame now.
fn frame_now(state: &CardState, effort: &Effort, fast_on: bool, cx: &App) -> Frame {
    state.motion.borrow_mut().frame(
        Input {
            stops: effort.steps.len(),
            current: effort.current,
            maximum: effort.maximum(),
            fast: fast_on,
            // On a software renderer the app holds its motion still
            // (`motion::note_renderer`): the colours change, nothing moves.
            reduce: cx.reduce_motion(),
        },
        state.now(),
    )
}

/// What the title row shows.
struct Title<'a> {
    effort: &'a Effort,
    shown: Option<usize>,
    model_name: &'a str,
    warning: Option<f32>,
    at_maximum: bool,
    touch: bool,
}

/// The title row: the bolt, the word over the model's name as one button,
/// the reset; while the slider is dragged onto the top stop, the warning
/// over all of it.
fn title_row(
    shell: &Shell,
    title: &Title<'_>,
    tokens: &Tokens,
    cx: &mut Context<'_, Shell>,
) -> Div {
    let neutral = tokens.neutral;
    let words = shell.t.lang.options();
    let hidden = title.warning.is_some();
    let quiet = neutral.foreground_extra_muted;
    let mut row = div()
        .relative()
        .min_h(px(TITLE))
        .px(px(TITLE_SIDE))
        .flex()
        .items_center()
        .justify_center();
    if let (Some(fast), false) = (title.effort.fast.as_ref(), hidden) {
        let choice = fast.choose(!fast.on());
        let (which, colour) = match (fast.on(), title.at_maximum) {
            (true, true) => (Glyph::LightningFilled, purple(tokens)),
            (true, false) => (Glyph::LightningFilled, accent_mark(tokens)),
            (false, _) => (Glyph::Lightning, quiet),
        };
        let bolt = corner("effort-fast", which, BOLT, colour, tokens)
            .left_0()
            .on_click(cx.listener(move |shell, _event, _window, cx| {
                shell.choose_option(choice.clone());
                cx.notify();
            }));
        let tip = format!("{}\n{}", words.fast_speed, words.fast_usage);
        row = row.child(shell.areas.draft.features.tips.on(
            bolt,
            &shell.areas.draft.effort.bolt,
            tip,
            title.touch,
        ));
    }
    if !hidden {
        row = row.child(
            corner("effort-reset", Glyph::Reset, 16.0, quiet, tokens)
                .right_0()
                .on_click(cx.listener(|shell, _event, _window, cx| {
                    shell.effort_choices(Effort::reset);
                    cx.notify();
                })),
        );
    }
    let toggle = toggle(title, hidden, tokens, words.default_model);
    let toggle = if hidden {
        toggle
    } else {
        toggle.on_click(cx.listener(|shell, _event, window, cx| {
            shell.areas.draft.effort.set_open(false);
            shell.open_picker(Picker::Model, window, cx);
        }))
    };
    row = row.child(toggle);
    if let Some(since) = title.warning {
        row = row.child(warning(words.maximum_warning, since, tokens));
    }
    row
}

/// The title's one button: the effort's word over the model's name and a
/// chevron, the hover on its content.
fn toggle(
    title: &Title<'_>,
    hidden: bool,
    tokens: &Tokens,
    default_word: &'static str,
) -> vessel_app_kit::ui::Stateful<Div> {
    let neutral = tokens.neutral;
    let quiet = neutral.foreground_extra_muted;
    let hover = neutral.surface3;
    div()
        .id("effort-model")
        .min_w_0()
        .min_h(px(SQUARE))
        .p(px(4.0))
        .rounded(px(8.0))
        .cursor_pointer()
        .group("effort-title")
        .when(hidden, |toggle| toggle.opacity(0.0))
        .child(
            div()
                .min_w_0()
                .px(px(4.0))
                .py(px(2.0))
                .rounded(px(6.0))
                .flex()
                .flex_col()
                .items_center()
                .group_hover("effort-title", move |style| style.bg(hover))
                .child(
                    div()
                        .type_(Type::Label)
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(word_colour(title.effort, title.shown, tokens))
                        .child(
                            word(title.effort, title.shown)
                                .unwrap_or(default_word)
                                .to_owned(),
                        ),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(2.0))
                        .type_(Type::Meta)
                        .text_color(neutral.foreground_muted)
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .child(title.model_name.to_owned()),
                        )
                        .child(glyph(Glyph::ChevronRight, 12.0, quiet)),
                ),
        )
}

/// A 32-point corner button of one glyph with a 26-point hover square.
fn corner(
    id: &'static str,
    which: Glyph,
    size: f32,
    colour: Hsla,
    tokens: &Tokens,
) -> vessel_app_kit::ui::Stateful<Div> {
    let hover = tokens.neutral.surface3;
    div()
        .id(id)
        .absolute()
        .top_0()
        .size(px(SQUARE))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .group(id)
        .child(
            div()
                .size(px(SQUARE_HOVER))
                .rounded(px(7.0))
                .flex()
                .items_center()
                .justify_center()
                .group_hover(id, move |style| style.bg(hover))
                .child(glyph(which, size, colour)),
        )
}

/// The usage warning over the title row, purple, a light band crossing it
/// once (`view::slider::flair::shimmer`): one run of text, each
/// character's colour a highlight, so the words keep their shaping.
fn warning(text: &'static str, since: f32, tokens: &Tokens) -> Div {
    let base = purple(tokens);
    let ink = tokens.neutral.foreground;
    let count = text.chars().count().max(1);
    let last = vessel_app_view::slider::float(count.saturating_sub(1).max(1));
    let highlights: Vec<(std::ops::Range<usize>, gpui::HighlightStyle)> = text
        .char_indices()
        .enumerate()
        .map(|(index, (start, character))| {
            let at = vessel_app_view::slider::float(index).mul_add(last.recip(), 0.0);
            let colour = base.blend(ink.opacity(flair::shimmer(at, since)));
            (
                start..start.saturating_add(character.len_utf8()),
                gpui::HighlightStyle {
                    color: Some(colour),
                    ..gpui::HighlightStyle::default()
                },
            )
        })
        .collect();
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .type_(Type::Meta)
        .whitespace_nowrap()
        .text_color(base)
        .child(gpui::StyledText::new(text).with_highlights(highlights))
}

/// The kit's paint for `frame`.
fn paint(frame: &Frame, tokens: &Tokens) -> Paint {
    let accent = tokens.neutral.accent;
    let violet = purple(tokens);
    let field = frame
        .field
        .map(|(time, reveal)| sampler(accent, violet, time, reveal));
    Paint {
        fraction: frame.fraction,
        thumb: frame.thumb,
        stops: frame
            .stops
            .iter()
            .map(|(dot, filled)| Stop {
                shift: dot.x,
                scale: dot.scale,
                opacity: dot.opacity,
                filled: *filled,
            })
            .collect(),
        burst: frame
            .burst
            .iter()
            .map(|(dot, blue)| Particle {
                x: dot.x,
                y: dot.y,
                scale: dot.scale,
                opacity: dot.opacity,
                colour: violet.blend(accent.opacity(*blue)),
            })
            .collect(),
        dots: frame
            .dots
            .iter()
            .map(|dot| Particle {
                x: dot.x,
                y: dot.y,
                scale: dot.scale,
                opacity: dot.opacity.mul_add(0.72, 0.0),
                colour: gpui::white(),
            })
            .collect(),
        field,
        fill: accent,
        moving: frame.moving,
        ring: None,
    }
}

/// The colour channels of `colour`.
fn channels(colour: Hsla) -> flair::Rgb {
    let rgba = colour.to_rgb();
    [rgba.r, rgba.g, rgba.b]
}

/// The top stop's field: the Codex app's moving blobs giving their light
/// to the accent-to-purple gradient, revealed from the thumb.
fn sampler(accent: Hsla, violet: Hsla, time: f32, reveal: f32) -> Sampler {
    let tone = Tone {
        field: flair::Field::at(time),
        time,
        reveal,
        blue: channels(accent),
        purple: channels(violet),
    };
    Rc::new(move |x, up, fill, width| tone.sample(x, up, fill, width))
}

/// One frame's field and the colours it lights.
#[derive(Clone, Copy)]
struct Tone {
    field: flair::Field,
    time: f32,
    reveal: f32,
    blue: flair::Rgb,
    purple: flair::Rgb,
}

impl Tone {
    #[expect(
        clippy::float_arithmetic,
        reason = "one sample of the field: a point in the fill to the field's own coordinates"
    )]
    fn sample(&self, x: f32, up: f32, fill: f32, width: f32) -> [f32; 4] {
        // The reference's field spans the whole track, its end under the
        // thumb: a point `x` into the fill is `x + width - fill` into it.
        let into = x + width - fill;
        let u = if width > 0.0 { into / width } else { 0.0 };
        let cell = vessel_app_view::slider::float(usize::try_from(whole(into / 2.0)).unwrap_or(0))
            + vessel_app_view::slider::float(usize::try_from(whole(up * 12.0)).unwrap_or(0))
                * 997.0;
        let grain = flair::hash(0, cell + self.time) * 2.0 - 1.0;
        let lit = self.field.colour(u, up, grain);
        let [red, green, blue] = flair::luminosity(flair::backdrop(u, self.blue, self.purple), lit);
        [red, green, blue, flair::reveal(into, width, self.reveal)]
    }
}

/// `value` rounded down to a whole non-negative number.
#[expect(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a sample's cell in a track a few hundred points wide"
)]
fn whole(value: f32) -> u32 {
    value.clamp(0.0, 65_535.0).floor() as u32
}
